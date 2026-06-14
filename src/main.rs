#[allow(unused_imports)]
use std::io::{self, Write};

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
                println!("{}: command not found", command);
            }
            Err(e) => println!("shell: read error: {}", e),
        }
    }
}
