use clap::{Parser, Subcommand};
use tftp::{ClienteTFTP,Status};
use std::{net::Ipv4Addr, process::exit};

/// Um pequeno cliente TFTP experimental
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
   /// Nome ou endereço IP do servidor
   #[arg(short, long)]
   server: String,

   /// Port do servidor (default: 69)
   #[arg(short, long, default_value_t = 69)]
   port: u16,

   /// comando: envia ou recebe
   #[command(subcommand)]
   cmd: Comandos,
}

#[derive(Debug)]
#[derive(Subcommand)]
enum Comandos {
   envia {arquivo: String},
   recebe {arquivo: String} 
}

fn main() {
   let args = Args::parse();

   match &args.cmd {
      Comandos::envia{arquivo} => {
         println!("enviando {} para {}", arquivo, &args.server);
      }
      Comandos::recebe {arquivo} => {
         println!("recebendo {} de {}", arquivo, &args.server);
      }
   }
   std::process::exit(0);

   let cliente = ClienteTFTP::new(&args.server, args.port);
   match cliente.recebe("teste", "teste") {
      Status::OK => println!("Arquivo recebido e gravado"),
      Status::Error(e) => println!("Erro: {:?}", e),
      Status::Unknown => println!("Erro desconhecido"),
      Status::Timeout => println!("Timeout"),
      Status::MaxRetriesExceeded => println!("retransmissões excedidas")
   }
}