use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;

use arinc429_sniffer::core::word::WordEndianness;
use arinc429_sniffer::core::types::PayloadFormat;
use arinc429_sniffer::io::reader::{ArincDumpReader, DumpFormat};
use arinc429_sniffer::io::pipeline::WordPipeline;
use arinc429_sniffer::ui::printer::{DisplayOptions, OutputMode, OutputPrinter};
use arinc429_sniffer::core::dictionary::get_avionics_dictionary;

#[derive(Parser, Debug)]
#[command(
    name = "a429-sniff",
    version = "0.1.0",
    author = "Aerospace Systems Lab",
    about = "ARINC 429 Aviation Bus Packet Analyzer - Geek Edition",
    long_about = "High-precision ARINC 429 bus sniffer with BNR/BCD decoding\n\
                 Supports raw binary dumps, hex text, and PCAP formats.\n\
                 Avionics dictionary included for standard labels.",
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,

    #[arg(long = "no-color", default_value_t = false)]
    no_color: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "decode", about = "Decode ARINC 429 bus dump file")]
    Decode(DecodeArgs),

    #[command(name = "analyze", about = "Analyze bus traffic and generate statistics report")]
    Analyze(DecodeArgs),

    #[command(name = "dict", about = "Show built-in avionics dictionary / label definitions")]
    Dict(DictArgs),

    #[command(name = "gendata", about = "Generate synthetic test data for validation")]
    GenData(GenDataArgs),

    #[command(name = "info", about = "Display build info and supported features")]
    Info,
}

#[derive(clap::Args, Debug)]
struct DecodeArgs {
    #[arg(value_name = "FILE", help = "Binary dump file path")]
    file: PathBuf,

    #[arg(short = 'f', long = "format", value_enum, default_value_t = FormatArg::Auto)]
    format: FormatArg,

    #[arg(short = 'e', long = "endian", value_enum, default_value_t = EndianArg::Le)]
    endian: EndianArg,

    #[arg(short = 'm', long = "mode", value_enum, default_value_t = ModeArg::Pretty)]
    mode: ModeArg,

    #[arg(short = 'n', long = "limit", help = "Limit number of words to display")]
    limit: Option<usize>,

    #[arg(long = "skip-parity", default_value_t = false, help = "Skip words with parity errors")]
    skip_parity: bool,

    #[arg(long = "known-only", default_value_t = false, help = "Only decode known (defined) labels")]
    known_only: bool,

    #[arg(long = "raw", default_value_t = false, help = "Show raw hex/bit details")]
    raw: bool,

    #[arg(long = "bits", default_value_t = false, help = "Show bitwise breakdown")]
    bits: bool,

    #[arg(long = "no-stats", default_value_t = false, help = "Suppress statistics output")]
    no_stats: bool,

    #[arg(short = 'L', long = "label", help = "Filter by octal label (e.g. 001, 030, 377)")]
    label_filter: Vec<String>,
}

#[derive(clap::Args, Debug)]
struct DictArgs {
    #[arg(help = "Filter by label (octal) or keyword")]
    query: Option<String>,

    #[arg(long = "format", help = "Filter by format: BNR, BCD, Discrete, Maintenance")]
    format: Option<String>,
}

#[derive(clap::Args, Debug)]
struct GenDataArgs {
    #[arg(short = 'o', long = "output", help = "Output file path")]
    output: PathBuf,

    #[arg(short = 'c', long = "count", default_value_t = 1000, help = "Number of words to generate")]
    count: usize,

    #[arg(short = 'f', long = "format", value_enum, default_value_t = OutFormatArg::Bin)]
    format: OutFormatArg,

    #[arg(long = "seed", help = "Random seed for reproducible output")]
    seed: Option<u64>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum FormatArg {
    Auto,
    Bin,
    Hex,
    Pcap,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum EndianArg {
    Le,
    Be,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ModeArg {
    Pretty,
    Compact,
    Hex,
    Bits,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutFormatArg {
    Bin,
    Hex,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {}", "ERROR".red().bold(), e);
            for (i, cause) in e.chain().skip(1).enumerate() {
                eprintln!("  {} Caused by: {}", i + 1, cause);
            }
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        colored::control::set_override(false);
    }

    match cli.command {
        Commands::Decode(args) => cmd_decode(args, cli.no_color, false),
        Commands::Analyze(args) => cmd_decode(args, cli.no_color, true),
        Commands::Dict(args) => cmd_dict(args, cli.no_color),
        Commands::GenData(args) => cmd_gendata(args),
        Commands::Info => cmd_info(cli.no_color),
    }
}

fn cmd_decode(args: DecodeArgs, no_color: bool, stats_only: bool) -> Result<()> {
    let output_mode = match args.mode {
        ModeArg::Pretty => OutputMode::Pretty,
        ModeArg::Compact => OutputMode::Compact,
        ModeArg::Hex => OutputMode::RawHex,
        ModeArg::Bits => OutputMode::Bitwise,
    };

    let display_opts = DisplayOptions {
        mode: output_mode,
        show_raw: args.raw || (matches!(output_mode, OutputMode::Pretty) && args.raw),
        show_bits: args.bits || matches!(output_mode, OutputMode::Bitwise),
        show_desc: true,
        color: !no_color,
        line_limit: args.limit,
    };

    let dump_format = match args.format {
        FormatArg::Auto => {
            let ext = args.file.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            match ext.as_str() {
                "hex" | "txt" | "log" => DumpFormat::HexText,
                "pcap" | "cap" => DumpFormat::PcapLe,
                _ => DumpFormat::RawBinary,
            }
        }
        FormatArg::Bin => DumpFormat::RawBinary,
        FormatArg::Hex => DumpFormat::HexText,
        FormatArg::Pcap => DumpFormat::PcapLe,
    };

    let endianness = match args.endian {
        EndianArg::Le => WordEndianness::Standard,
        EndianArg::Be => WordEndianness::Reversed,
    };

    let reader = ArincDumpReader::new(&args.file)
        .with_format(dump_format)
        .with_endianness(endianness);

    if !stats_only {
        let printer = OutputPrinter::new(display_opts.clone());
        printer.print_banner();
        printer.print_dictionary_size();
        println!();
    }

    let (words, reader_stats) = reader.read_all()
        .with_context(|| format!("Failed to read dump file: {}", args.file.display()))?;

    let pipeline = WordPipeline::new()
        .skip_parity_invalid(args.skip_parity)
        .only_known_labels(args.known_only);

    let label_octal_filters: Vec<u16> = args.label_filter.iter()
        .filter_map(|s| u16::from_str_radix(s, 8).ok())
        .collect();

    let (decoded, pipeline_stats) = if label_octal_filters.is_empty() {
        pipeline.process_all(words)
    } else {
        let filtered: Vec<_> = words.into_iter()
            .filter(|w| label_octal_filters.contains(&w.label_octal()))
            .collect();
        let total = filtered.len();
        let (dec, mut stats) = pipeline.process_all(filtered);
        stats.total_words = total;
        (dec, stats)
    };

    if stats_only {
        if !no_color {
            println!("{}", "ARINC 429 TRAFFIC ANALYSIS REPORT".bold().cyan());
            println!("{}", "═".repeat(60).dimmed());
        } else {
            println!("ARINC 429 TRAFFIC ANALYSIS REPORT");
            println!("{}", "═".repeat(60));
        }
        println!();
        println!("{}", reader_stats.to_string());
        println!("{}", pipeline_stats.summary());
    } else {
        let mut printer = OutputPrinter::new(display_opts.clone());
        if matches!(output_mode, OutputMode::Compact) {
            let hdr = if no_color {
                format!("{:>6} | {:>3} | {:>1} | {:>6} | {:>8} | {:>2} | {:<10} | {}",
                    "IDX", "LBL", "SDI", "SSM", "DATA", "P", "EQUIP", "VALUE")
            } else {
                format!("{:>6} | {:>3} | {:>1} | {:>6} | {:>8} | {:>2} | {:<10} | {}",
                    "IDX".bold(), "LBL".bold().yellow(), "SDI".bold(),
                    "SSM".bold(), "DATA".bold().cyan(), "P".bold(),
                    "EQUIP".bold(), "ENGINEERING VALUE".bold().green())
            };
            println!("{}", hdr);
            println!("{:-<1$}", "", 100);
        }
        printer.print_decoded_all(&decoded);

        if !args.no_stats {
            printer.print_reader_stats(&reader_stats);
            printer.print_pipeline_stats(&pipeline_stats);
        }
    }

    Ok(())
}

fn cmd_dict(args: DictArgs, no_color: bool) -> Result<()> {
    let dict = get_avionics_dictionary();

    if !no_color {
        println!("{}", "AVIONICS LABEL DICTIONARY".bold().yellow());
        println!("  Total definitions: {}\n", dict.len().to_string().bold());
    } else {
        println!("AVIONICS LABEL DICTIONARY");
        println!("  Total definitions: {}\n", dict.len());
    }

    let format_filter = args.format.as_ref().map(|s| s.to_lowercase());
    let query = args.query.as_ref().map(|s| s.to_lowercase());

    let mut entries: Vec<_> = dict.values().collect();
    entries.sort_by_key(|d| d.label_octal);

    let mut count = 0usize;

    for def in entries {
        if let Some(ref fmt) = format_filter {
            let fmt_str = match def.format {
                PayloadFormat::Bnr => "bnr",
                PayloadFormat::Bcd => "bcd",
                PayloadFormat::Discrete => "discrete",
                PayloadFormat::Maintenance => "maintenance",
                _ => continue,
            };
            if fmt_str != fmt.as_str() {
                continue;
            }
        }

        if let Some(ref q) = query {
            let mut haystack = String::new();
            haystack.push_str(&format!("{:03o}", def.label_octal));
            haystack.push(' ');
            haystack.push_str(&def.param_name.to_lowercase());
            haystack.push(' ');
            haystack.push_str(&def.equipment.to_lowercase());
            haystack.push(' ');
            haystack.push_str(&def.description.to_lowercase());

            if !haystack.contains(q.as_str()) {
                continue;
            }
        }

        let fmt_tag_str = match def.format {
            PayloadFormat::Bnr => "[BNR]",
            PayloadFormat::Bcd => "[BCD]",
            PayloadFormat::Discrete => "[DSC]",
            PayloadFormat::Maintenance => "[MAINT]",
            PayloadFormat::Ack => "[ACK]",
            PayloadFormat::Unknown => "[???]",
        };

        let fmt_tag_c = if !no_color {
            match def.format {
                PayloadFormat::Bnr => fmt_tag_str.blue().bold().to_string(),
                PayloadFormat::Bcd => fmt_tag_str.magenta().bold().to_string(),
                PayloadFormat::Discrete => fmt_tag_str.yellow().bold().to_string(),
                _ => fmt_tag_str.dimmed().to_string(),
            }
        } else {
            fmt_tag_str.to_string()
        };

        if !no_color {
            println!(
                "  {:>03o} {} {:<22} {:<8} {:>10.6} {:<8}  {}",
                def.label_octal,
                fmt_tag_c,
                def.param_name.bold(),
                def.equipment.dimmed(),
                def.resolution,
                def.unit,
                def.description
            );
        } else {
            println!(
                "  {:>03o} {} {:<22} {:<8} {:>10.6} {:<8}  {}",
                def.label_octal, fmt_tag_c, def.param_name, def.equipment,
                def.resolution, def.unit, def.description
            );
        }

        count += 1;
    }

    println!();
    if no_color {
        println!("  {} label(s) matched", count);
    } else {
        println!("  {} label(s) matched", count.to_string().bold().green());
    }

    Ok(())
}

fn cmd_gendata(args: GenDataArgs) -> Result<()> {
    use std::fs::File;
    use std::io::Write;
    use rand::Rng;
    use rand::SeedableRng;

    use arinc429_sniffer::core::word::{
        SDI_MASK, DATA_MASK, SSM_MASK, PARITY_MASK,
        LABEL_SHIFT, SDI_SHIFT, DATA_SHIFT, SSM_SHIFT,
    };

    println!("Generating {} synthetic ARINC 429 words -> {}",
        args.count, args.output.display());

    let mut rng = match args.seed {
        Some(s) => rand::rngs::StdRng::seed_from_u64(s),
        None => rand::rngs::StdRng::from_entropy(),
    };

    let know_labels: Vec<u16> = vec![
        0o001, 0o002, 0o003, 0o004, 0o005, 0o010, 0o011, 0o012, 0o013,
        0o020, 0o021, 0o022, 0o030, 0o033, 0o040, 0o041, 0o042, 0o043,
        0o050, 0o060, 0o072, 0o100, 0o102, 0o103, 0o104, 0o105, 0o377,
    ];

    let mut raw_words: Vec<u32> = Vec::with_capacity(args.count);

    for _ in 0..args.count {
        let use_known = rng.gen_bool(0.75);

        let label_val: u8 = if use_known {
            let idx = rng.gen_range(0..know_labels.len());
            know_labels[idx] as u8
        } else {
            rng.gen::<u8>()
        };

        let sdi_val: u8 = rng.gen_range(0..4);
        let data_val: u32 = rng.gen_range(0..(1u32 << 19));
        let ssm_val: u8 = rng.gen_range(0..4);

        let mut raw: u32 = 0;
        raw |= (label_val as u32) << LABEL_SHIFT;
        raw &= !SDI_MASK;
        raw |= (sdi_val as u32) << SDI_SHIFT;
        raw &= !DATA_MASK;
        raw |= (data_val << DATA_SHIFT) & DATA_MASK;
        raw &= !SSM_MASK;
        raw |= (ssm_val as u32) << SSM_SHIFT;

        let ones = raw.count_ones();
        if ones % 2 == 0 {
            raw |= PARITY_MASK;
        }

        raw_words.push(raw);
    }

    match args.format {
        OutFormatArg::Bin => {
            let mut f = File::create(&args.output)
                .with_context(|| format!("Cannot create {}", args.output.display()))?;
            for w in &raw_words {
                f.write_all(&w.to_le_bytes())?;
            }
        }
        OutFormatArg::Hex => {
            let mut f = File::create(&args.output)
                .with_context(|| format!("Cannot create {}", args.output.display()))?;
            writeln!(f, "# Synthetic ARINC 429 test dump")?;
            writeln!(f, "# {} words, seed: {:?}", args.count, args.seed)?;
            writeln!(f, "# Format: HEX (8 digits per line, little-endian)")?;
            writeln!(f)?;
            for w in &raw_words {
                writeln!(f, "{:08X}", w)?;
            }
        }
    }

    let sz = std::fs::metadata(&args.output)?.len();
    println!("Done! {:.2} MB written ({} bytes)",
        sz as f64 / 1048576.0, sz);

    Ok(())
}

fn cmd_info(no_color: bool) -> Result<()> {
    let dict = get_avionics_dictionary();

    let bnr_count = dict.values().filter(|d| matches!(d.format, PayloadFormat::Bnr)).count();
    let bcd_count = dict.values().filter(|d| matches!(d.format, PayloadFormat::Bcd)).count();
    let disc_count = dict.values().filter(|d| matches!(d.format, PayloadFormat::Discrete)).count();
    let sys_count = dict.values().filter(|d| matches!(d.format, PayloadFormat::Maintenance | PayloadFormat::Ack)).count();

    let title = "ARINC 429 BUS SNIFFER - SYSTEM INFORMATION";
    if !no_color {
        println!("{}", title.bold().cyan());
        println!("{}", "─".repeat(60).dimmed());
    } else {
        println!("{}", title);
        println!("{}", "─".repeat(60));
    }

    println!();
    println!("  Build:          a429-sniff v{}", env!("CARGO_PKG_VERSION"));
    println!("  Rust Toolchain: compiled with stable Rust");
    println!("  Target:         Linux / x86_64 / musl-libc");
    println!();
    println!("  Dictionary:     {} total labels", dict.len());
    println!("     + BNR (Binary):       {} definitions", bnr_count);
    println!("     + BCD (Decimal):      {} definitions", bcd_count);
    println!("     + Discrete:           {} definitions", disc_count);
    println!("     + System/Maint:       {} definitions", sys_count);
    println!();
    println!("  Decoding Engine:");
    println!("     + Word-level bitwise extraction  (1-8 LBL | 9-10 SDI | 11-29 DATA | 30-31 SSM | 32 P)");
    println!("     + BNR two's complement decode  (signed magnitude via SSM +/-)");
    println!("     + BCD digit extraction         (4-bit groups, octal support)");
    println!("     + Discrete bitmask decoding    (named flags per position)");
    println!();
    println!("  Input Formats:    Raw Binary | Hex Text | PCAP | PCAP-BE");
    println!("  Output Modes:     Pretty | Compact | Hex-Raw | Bitwise");
    println!("  Performance:      mem-mapped buffered I/O, zero-copy parsing");

    Ok(())
}
