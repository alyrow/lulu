mod install;
mod remove;
mod setup;
mod update;
mod upgrade;

pub fn install(name: Option<String>, no_install: bool) {
    install::install(name, no_install)
}

pub fn setup() {
    setup::setup()
}

pub fn update(no_check: bool) {
    update::update(no_check)
}

pub fn upgrade() {
    upgrade::upgrade()
}

pub fn remove(name: String, purge: bool) {
    remove::remove(name, purge)
}
