use std::env;
use std::fs::File;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio, exit};

struct Cmd {
    command: String,
    parameter: Vec<String>,
    input_source: String,
    out_source: String,
}

struct Pipeline {
    pipeline: Vec<Cmd>,
}

enum LogicExpr {
    Base(Pipeline),
    And(Box<LogicExpr>, Box<LogicExpr>),
    // Or(Box<LogicExpr>, Box<LogicExpr>),
}

enum Builtin {
    Exit,
    Echo,
    Type,
    PWD,
}

impl Builtin {
    fn from_str(s: &str) -> Option<Builtin> {
        match s {
            "exit" => Some(Builtin::Exit),
            "echo" => Some(Builtin::Echo),
            "type" => Some(Builtin::Type),
            "pwd" => Some(Builtin::PWD),
            _ => None,
        }
    }
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!("\nexit");
                break;
            }
            Ok(_) => {
                let input = input.trim();
                let tree = parser(&tokenize(input));
                execute_logic(tree);
            }
            Err(e) => println!("shell: read error: {}", e),
        }
    }
}

fn execute_logic(expr: LogicExpr) -> i32 {
    match expr {
        LogicExpr::Base(pipeline) => execute_pipeline(pipeline),
        LogicExpr::And(left, right) => {
            let status = execute_logic(*left);
            if status == 0 {
                execute_logic(*right)
            } else {
                status
            }
        } // LogicExpr::Or(left, right) => todo!(),
    }
}

fn find_in_path(cmd: &str) -> Option<std::path::PathBuf> {
    let env_path = std::env::var("PATH").ok()?;

    std::env::split_paths(&env_path).find_map(|dir| {
        let full_path = dir.join(cmd);

        if let Ok(metedata) = full_path.metadata() {
            if metedata.is_file() && (metedata.permissions().mode() & 0o111 != 0) {
                return Some(full_path);
            }
        }
        None
    })
}

fn execute_pipeline(pipeline_obj: Pipeline) -> i32 {
    let mut last_status = 0;

    for cmd in pipeline_obj.pipeline {
        match Builtin::from_str(cmd.command.as_str()) {
            Some(builtin) => match builtin {
                Builtin::Exit => {
                    println!("exit");
                    exit(0);
                }
                Builtin::Echo => {
                    for (i, parameter) in cmd.parameter.iter().enumerate() {
                        if i > 0 {
                            print!(" ");
                        }
                        print!("{}", parameter);
                    }
                    println!();
                }
                Builtin::Type => {
                    let arg: &String;
                    if cmd.parameter.len() > 0 {
                        arg = &cmd.parameter[0];
                    } else {
                        return 1;
                    }

                    if Builtin::from_str(arg).is_some() {
                        println!("{} is a shell builtin", arg);
                    } else if let Some(full_path) = find_in_path(arg) {
                        println!("{} is {}", arg, full_path.display());
                    } else {
                        println!("{}: not found", arg);
                    }
                }
                Builtin::PWD => {
                    match env::current_dir() {
                        Ok(s) => println!("{}", s.display()),
                        Err(_) => last_status = 1,
                    }
                }
            },
            None => {
                let mut child_cmd = Command::new(&cmd.command);
                child_cmd.args(&cmd.parameter);

                if !cmd.input_source.is_empty() {
                    let file = File::open(&cmd.input_source).expect("error");
                    child_cmd.stdin(Stdio::from(file));
                } else {
                    child_cmd.stdin(Stdio::inherit());
                }

                if !cmd.out_source.is_empty() {
                    let file = File::create(&cmd.out_source).expect("error");
                    child_cmd.stdout(Stdio::from(file));
                } else {
                    child_cmd.stdout(Stdio::inherit());
                }

                child_cmd.stderr(Stdio::inherit());

                match child_cmd.status() {
                    Ok(status) => {
                        last_status = status.code().unwrap_or(0);
                    }
                    Err(_) => {
                        eprintln!("{}: command not found", cmd.command);
                        last_status = 127;
                    }
                }
            }
        }
    }
    last_status
}

/// Processes a token.
///
/// **NOTE:** `token` must be trimmed before passing in.
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let rest = input.as_bytes();
    let mut pos = 0;

    while pos < rest.len() {
        if rest[pos] == b' ' || rest[pos] == b'\t' {
            pos += 1;
            continue;
        }

        let start = pos;
        while pos < rest.len() && rest[pos] != b' ' && rest[pos] != b'\t' {
            pos += 1;
        }
        tokens.push(input[start..pos].to_string());
    }

    tokens
}

fn parse_to_pipeline(tokens: &[String]) -> Pipeline {
    let mut pipeline_obj = Pipeline {
        pipeline: Vec::new(),
    };
    let parts: Vec<&[String]> = tokens.split(|x| x == "|").collect();

    for part in parts {
        if part.is_empty() {
            continue;
        }

        let mut cmd = Cmd {
            command: part[0].clone(),
            parameter: Vec::new(),
            input_source: String::new(),
            out_source: String::new(),
        };

        let mut i = 1;
        while i < part.len() {
            match part[i].as_str() {
                ">" => {
                    if i + 1 < part.len() {
                        cmd.out_source = part[i + 1].clone();
                        i += 2;
                    }
                }
                "<" => {
                    if i + 1 < part.len() {
                        cmd.input_source = part[i + 1].clone();
                        i += 2;
                    }
                }
                _ => {
                    cmd.parameter.push(part[i].clone());
                    i += 1;
                }
            }
        }
        pipeline_obj.pipeline.push(cmd);
    }

    pipeline_obj
}

fn parser(tokens: &[String]) -> LogicExpr {
    if let Some(pos) = tokens.iter().rposition(|x| x == "&&") {
        let left = &tokens[..pos];
        let right = &tokens[pos + 1..];

        let left_expr = parser(left);
        let right_expr = parser(right);

        LogicExpr::And(Box::new(left_expr), Box::new(right_expr))
    } else {
        let p = parse_to_pipeline(tokens);
        LogicExpr::Base(p)
    }
}
