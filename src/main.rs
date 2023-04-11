use clap::Parser;
use tftp::{ClienteTFTP,Status};

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
   let cliente = ClienteTFTP::new(&args.server, args.port);
   match cliente.recebe("teste", "teste") {
      Status::OK => println!("Arquivo recebido e gravado"),
      Status::Error(e) => println!("Erro: {:?}", e),
      Status::Unknown => println!("Erro desconhecido"),
      Status::Timeout => println!("Timeout"),
      Status::MaxRetriesExceeded => println!("retransmissões excedidas")
   }
}