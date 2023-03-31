use clap::Parser;
mod msg;
use std::fs;

/// Um pequeno cliente TFTP experimental
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
   /// Name of the person to greet
   #[arg(short, long)]
   server: String,

   /// Number of times to greet
   #[arg(short, long, default_value_t = 69)]
   port: u16,
}

fn main() {
   let args = Args::parse();
   let buffer = fs::read(args.server).expect("não conseguiu abrir ou ler");
   let m1 = msg::from_bytes(buffer).expect("msg inválida");

   println!("Mensagem -> {}", m1);
   // println!("Server: {}:{}", args.server, args.port);
}