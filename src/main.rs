#[allow(unused_imports)]
use std::io::{self, Write};
use std::process::exit;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        match io::stdin().read_line(&mut command) {
            Ok(0) => {
                println!("\nexit");
                break;
            }
            Ok(_) => {
                let command = command.trim();
                exec(command);
            }
            Err(e) => println!("shell: read error: {}", e),
        }
    }
}

fn exec(command: &str) {
    match command {
        "" => {}
        "exit" => {
            println!("exit");
            exit(0);
        }
        _ => {
            println!("{}: command not found", command);
        }
    }
}
