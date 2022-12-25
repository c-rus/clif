use crate::cli::Cli;
use crate::error::CliError;
use std::fmt::Debug;

pub trait Command<T>: Debug {
    type Status;
    fn exec(&self, context: &T) -> Self::Status;
}

pub trait FromCli {
    /// Collects tokens from the command-line interface to define a struct's fields.
    ///
    /// The recommended argument discovery order is
    /// 1. `flags`
    /// 2. `optionals`
    /// 3. `positionals`
    /// 4. `subcommands`
    fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self, CliError<'c>>
    where
        Self: Sized;
}

pub trait Runner<T>: Command<T> + FromCli + Debug {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{arg::*, help::Help};

    /// Helper test fn to write vec of &str as iterator for Cli parameter.
    fn args<'a>(args: Vec<&'a str>) -> Box<dyn Iterator<Item = String> + 'a> {
        Box::new(args.into_iter().map(|f| f.to_string()).into_iter())
    }

    /// Example command to add two numbers together.
    #[derive(Debug, PartialEq)]
    struct Add {
        lhs: u32,
        rhs: u32,
        verbose: bool,
    }

    impl Command<()> for Add {
        type Status = ();

        fn exec(&self, _: &()) -> Self::Status {
            println!("{}", self.run())
        }
    }

    impl Add {
        /// Simple fn to return an answer for the `Add` test command.
        fn run(&self) -> String {
            let sum = self.lhs + self.rhs;
            match self.verbose {
                true => format!("{} + {} = {}", self.lhs, self.rhs, sum),
                false => format!("{}", sum),
            }
        }
    }

    impl FromCli for Add {
        fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self, CliError<'c>> {
            cli.help(
                Help::new()
                    .quick_text("    add <lhs> <rhs> [--verbose]")
                    .ref_usage(0..0),
            )?;
            // the ability to "learn options" beforehand is possible, or can be skipped
            // "learn options" here (take in known args (as ref?))
            Ok(Add {
                verbose: cli.check_flag(Flag::new("verbose"))?,
                lhs: cli.require_positional(Positional::new("lhs"))?,
                rhs: cli.require_positional(Positional::new("rhs"))?,
            })
        }
    }

    /// Tests a nested subcommand cli structure.
    #[derive(Debug, PartialEq)]
    struct Op {
        version: bool,
        command: Option<OpSubcommand>,
    }

    impl Command<()> for Op {
        type Status = ();

        fn exec(&self, context: &()) -> Self::Status {
            if let Some(command) = &self.command {
                command.exec(context)
            }
        }
    }

    impl FromCli for Op {
        fn from_cli<'c>(cli: &'c mut Cli<'_>) -> Result<Self, CliError<'c>> {
            Ok(Op {
                version: cli.check_flag(Flag::new("version"))?,
                command: cli.check_command(Positional::new("subcommand"))?,
            })
        }
    }

    #[derive(Debug, PartialEq)]
    enum OpSubcommand {
        Add(Add),
    }

    impl FromCli for OpSubcommand {
        fn from_cli<'c>(cli: &'c mut Cli<'_>) -> Result<Self, CliError<'c>> {
            match cli.match_command(&["add", "mult", "sub"])?.as_ref() {
                "add" => Ok(OpSubcommand::Add(Add::from_cli(cli)?)),
                _ => panic!("an unimplemented command was passed through!"),
            }
        }
    }

    impl Command<()> for OpSubcommand {
        type Status = ();
        fn exec(&self, _: &()) -> Self::Status {
            match self {
                OpSubcommand::Add(c) => c.exec(&()),
            }
        }
    }

    #[test]
    fn make_add_command() {
        let mut cli = Cli::new().tokenize(args(vec!["add", "9", "10"]));
        let add = Add::from_cli(&mut cli).unwrap();
        assert_eq!(
            add,
            Add {
                lhs: 9,
                rhs: 10,
                verbose: false
            }
        );

        let mut cli = Cli::new().tokenize(args(vec!["add", "1", "4", "--verbose"]));
        let add = Add::from_cli(&mut cli).unwrap();
        assert_eq!(
            add,
            Add {
                lhs: 1,
                rhs: 4,
                verbose: true
            }
        );

        let mut cli = Cli::new().tokenize(args(vec!["add", "5", "--verbose", "2"]));
        let add = Add::from_cli(&mut cli).unwrap();
        assert_eq!(
            add,
            Add {
                lhs: 5,
                rhs: 2,
                verbose: true
            }
        );
    }

    #[test]
    fn nested_commands() {
        let mut cli = Cli::new().tokenize(args(vec!["op", "add", "9", "10"]));
        let op = Op::from_cli(&mut cli).unwrap();
        assert_eq!(
            op,
            Op {
                version: false,
                command: Some(OpSubcommand::Add(Add {
                    lhs: 9,
                    rhs: 10,
                    verbose: false,
                }))
            }
        );

        let mut cli = Cli::new().tokenize(args(vec!["op"]));
        let op = Op::from_cli(&mut cli).unwrap();
        assert_eq!(
            op,
            Op {
                version: false,
                command: None
            }
        );

        let mut cli = Cli::new().tokenize(args(vec!["op", "--version", "add", "9", "10"]));
        let op = Op::from_cli(&mut cli).unwrap();
        assert_eq!(
            op,
            Op {
                version: true,
                command: Some(OpSubcommand::Add(Add {
                    lhs: 9,
                    rhs: 10,
                    verbose: false,
                }))
            }
        );

        // out-of-context arg '--verbose' move it after 'add'
        let mut cli = Cli::new().tokenize(args(vec!["op", "--verbose", "add", "9", "10"]));
        let op = Op::from_cli(&mut cli);
        assert!(op.is_err());
    }

    #[test]
    #[should_panic]
    fn unimplemented_nested_command() {
        let mut cli = Cli::new().tokenize(args(vec!["op", "mult", "9", "10"]));
        let _ = Op::from_cli(&mut cli);
    }
}
