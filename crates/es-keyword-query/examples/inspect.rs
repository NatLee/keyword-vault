mod support;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(input) = args.next() else {
        eprintln!("Usage: cargo run --example inspect -- '<expression>' [--raw] [--slop=20]");
        std::process::exit(2);
    };
    let mut precise = true;
    let mut slop = 20;
    for arg in args {
        if arg == "--raw" {
            precise = false;
        } else if let Some(value) = arg
            .strip_prefix("--slop=")
            .and_then(|s| s.parse::<u32>().ok())
        {
            slop = value;
        } else {
            eprintln!("Unknown option: {arg}");
            std::process::exit(2);
        }
    }
    let result = support::inspect(&input, precise, slop);
    println!(
        "{}",
        serde_json::to_string_pretty(&result).expect("serializable inspection")
    );
    if result["valid"] != true {
        std::process::exit(1);
    }
}
