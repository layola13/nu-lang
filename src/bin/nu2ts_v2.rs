// Nu2TS V2 测试程序
// 使用新的 Parser + Codegen 架构

use nu_compiler::nu2ts::{Nu2TsConverterV2, TsConfig};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: nu2ts_v2 <input.nu>");
        std::process::exit(1);
    }

    let input_file = &args[1];
    let nu_code = std::fs::read_to_string(input_file)?;

    let converter = Nu2TsConverterV2::new(TsConfig::default());
    let ts_code = converter.convert(&nu_code)?;

    println!("{}", ts_code);
    
    Ok(())
}
