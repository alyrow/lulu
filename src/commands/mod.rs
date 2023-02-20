mod install;

pub fn install(name: Option<String>, no_install: bool) {
    install::install(name, no_install)
}