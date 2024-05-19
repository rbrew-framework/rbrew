use argp::HelpStyle;

fn main() {
    cargo_rbrew::run(argp::parse_args_or_exit(&HelpStyle::default()))
}
