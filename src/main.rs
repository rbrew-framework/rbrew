// TODOS:
//
// - make `--help` output details of all possible platforms.
// - make `--help` output details of all possible output types.

#![feature(exitcode_exit_method)]

use argp::{FromArgValue, FromArgs, HelpStyle};
use std::{
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

mod tools;

fn graceful_error_exit(msg: impl Display) -> ! {
    eprintln!("Exit failure.\n{msg}");
    ExitCode::FAILURE.exit_process()
}

mod fields {
    use super::*;

    #[derive(Clone, Copy)]
    pub enum Platform {
        Gamecube,
    }

    impl Platform {
        pub fn target_json_name(self) -> &'static str {
            match self {
                Platform::Gamecube => "rbrew_gamecube.json",
            }
        }
    }

    impl FromArgValue for Platform {
        fn from_arg_value(value: &std::ffi::OsStr) -> Result<Self, String> {
            let str = value.to_str().ok_or("invalid UTF-8 string".to_string())?;
            Ok(match str {
                "gamecube" | "gc" => Self::Gamecube,
                _ => return Err("expected a valid platform.".to_string()),
            })
        }
    }

    #[derive(Default, Clone, Copy)]
    pub enum OutputType {
        #[default]
        Elf,
        Dol,
    }

    impl FromArgValue for OutputType {
        fn from_arg_value(value: &std::ffi::OsStr) -> Result<Self, String> {
            let str = value.to_str().ok_or("invalid UTF-8 string".to_string())?;
            Ok(match str {
                "elf" => Self::Elf,
                "dol" => Self::Dol,
                _ => return Err("expected a valid output type.".to_string()),
            })
        }
    }

    impl OutputType {
        pub fn supports_platform(self, platform: Platform) -> bool {
            #[allow(unreachable_patterns)]
            #[allow(clippy::match_like_matches_macro)]
            match (self, platform) {
                (Self::Elf, _) | (Self::Dol, Platform::Gamecube) => true,
                _ => false,
            }
        }

        pub fn extension_name(self) -> &'static str {
            match self {
                OutputType::Elf => "",
                OutputType::Dol => ".dol",
            }
        }
    }
}

/// The rbrew build subcommand.
#[derive(FromArgs)]
#[argp(subcommand, name = "build")]
struct RbrewCliSubBuild {
    /// The platform to build for.
    /// See `--help` for more details.
    #[argp(option)]
    platform: fields::Platform,
    /// Output file type.
    #[argp(option, default = "Default::default()")]
    output_type: fields::OutputType,
    /// Build all packages in the workspace.
    #[argp(switch)]
    workspace: bool,
    /// Builds the specific package in the workspace.
    #[argp(option)]
    package: Option<String>,
    /// Output directory.
    #[argp(option)]
    output_directory: Option<PathBuf>,
    /// Custom cargo flags.
    #[argp(option)]
    custom_options: Vec<String>,
}

/// The rbrew tools subommand.
#[derive(FromArgs)]
#[argp(subcommand, name = "tools")]
struct RbrewCliSubTools {}

#[derive(FromArgs)]
#[argp(subcommand)]
enum RbrewCliSub {
    Build(RbrewCliSubBuild),
    Tools(RbrewCliSubTools),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

impl Verbosity {
    fn should_output(self, level: Self) -> bool {
        self >= level
    }
}

impl Default for Verbosity {
    fn default() -> Self {
        Self::Normal
    }
}

impl FromArgValue for Verbosity {
    fn from_arg_value(value: &std::ffi::OsStr) -> Result<Self, String> {
        Ok(match value.to_str() {
            Some("quiet") => Self::Quiet,
            Some("normal") => Self::Normal,
            Some("verbose") => Self::Verbose,
            _ => return Err("expected 'quiet', 'normal' or 'verbose'.".to_string()),
        })
    }
}

/// The rbrew command.
#[derive(FromArgs)]
struct RbrewCli {
    /// Determines the verbosity of the stdout output.
    #[argp(option, default = "Default::default()")]
    verbosity: Verbosity,

    #[argp(subcommand)]
    subcommand: RbrewCliSub,
}

mod util {
    use super::*;

    pub fn cargo() -> Command {
        Command::new("cargo")
    }

    pub fn rbrew_file(path: String) -> Result<PathBuf, std::io::Error> {
        let path = PathBuf::from(path);
        if path.exists() {
            Ok(path)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("unable to find path '{path:?}'"),
            ))
        }
    }
}

fn main() {
    let cli: RbrewCli = argp::parse_args_or_exit(&HelpStyle::default());
    // let cli: RbrewCli = argp::cargo_parse_args_or_exit();

    match cli.subcommand {
        RbrewCliSub::Build(args) => build(args, cli.verbosity),
        RbrewCliSub::Tools(args) => tools(args, cli.verbosity),
    }
}

fn build(args: RbrewCliSubBuild, verbosity: Verbosity) {
    if !args.output_type.supports_platform(args.platform) {
        graceful_error_exit("output type does not support platform. See `--help`.")
    }

    let mut cmd = util::cargo();
    cmd.arg("build");
    if let Some(package) = &args.package {
        cmd.arg("--package").arg(package);
    }
    if args.workspace {
        cmd.arg("--workspace");
    }

    let target_json_ident = args.platform.target_json_name();
    let target_json = match util::rbrew_file(format!("targets/{target_json_ident}")) {
        Ok(ok) => ok,
        Err(err) => graceful_error_exit(format!(
            "failed to find the target json file for the platform: {err}"
        )),
    };

    cmd.arg(format!("--target={}", target_json.display()));

    for option in &args.custom_options {
        cmd.arg(option);
    }

    let mut status_cmd = Command::new(cmd.get_program());
    status_cmd.args(cmd.get_args());
    status_cmd.envs(cmd.get_envs().map(|env| (env.0, env.1.unwrap_or_default())));
    status_cmd.current_dir(cmd.get_current_dir().unwrap());

    let mut output_cmd = cmd;

    match verbosity {
        Verbosity::Quiet => {
            status_cmd.arg("--quiet");
        }
        Verbosity::Normal => {}
        Verbosity::Verbose => {
            status_cmd.arg("--verbose");
        }
    };
    let status = status_cmd
        .status()
        .expect("failed to execute cargo command");
    if !status.success() {
        graceful_error_exit("something went wrong when running cargo.")
    }

    let output = output_cmd
        .arg("-message-format=json")
        .arg("--quiet")
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("should never be possible if we succeeded before");
    }

    let utf8 = String::from_utf8(output.stdout).expect("expected valid UTF-8");
    let mut iter = utf8.chars().peekable();
    let mut jsons = vec![];
    loop {
        let mut i = 0usize;
        if iter.peek().is_none() {
            break;
        }
        let string: String = iter
            .by_ref()
            .take_while(|c| {
                match c {
                    '{' => i += 1,
                    '}' => i = i.saturating_sub(1),
                    _ => {}
                }
                i > 0
            })
            .collect();
        jsons.push(json::parse(&string).expect("expected valid json"))
    }

    let mut output_executable = vec![];
    for json in jsons {
        match json {
            json::JsonValue::Object(object) => {
                // if let Some(executable) = object.get() {}
                if let Some(executable) = object.get("executable") {
                    if let Some(str) = executable.as_str() {
                        output_executable.push(str.to_string())
                    }
                }
            }
            _ => panic!("expected json object"),
        }
    }

    for (gen, input) in output_executable.into_iter().enumerate() {
        let input = Path::new(&input);
        let output_dir = args
            .output_directory
            .clone()
            .unwrap_or(input.parent().map(Path::to_path_buf).unwrap_or_default());
        let output_gennerated_name = format!("output{gen}");
        let output_name = input
            .file_stem()
            .unwrap_or(OsStr::new(&output_gennerated_name));

        let mut output = output_dir.join(output_name);
        output.set_extension(args.output_type.extension_name());

        match args.output_type {
            fields::OutputType::Elf => {
                std::fs::copy(input, output).unwrap();
            }
            fields::OutputType::Dol => {
                tools::elf2dol(input, output).unwrap();
            }
        }
    }
}

fn tools(_args: RbrewCliSubTools, _verbosity: Verbosity) {}
