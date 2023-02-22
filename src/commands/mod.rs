mod install;
mod setup;
mod update;

pub fn install(name: Option<String>, no_install: bool) {
    install::install(name, no_install)
}

pub fn setup() {
    setup::setup()
}

pub fn update(no_check: bool) {
    update::update(no_check)
}
