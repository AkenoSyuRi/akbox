use is_terminal::IsTerminal;

pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}
