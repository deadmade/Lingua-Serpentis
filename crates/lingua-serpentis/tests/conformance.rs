mod support;

use libtest_mimic::{Arguments, Failed, Trial};

fn main() {
    let args = Arguments::from_args();
    let fixtures = support::discover();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {}",
        support::corpus_root().display()
    );
    let trials: Vec<Trial> = fixtures
        .into_iter()
        .map(|f| Trial::test(f.name.clone(), move || run(&f)))
        .collect();
    libtest_mimic::run(&args, trials).exit();
}

fn run(f: &support::Fixture) -> Result<(), Failed> {
    let got = support::run_fixture(f);
    if support::blessing() {
        support::bless(f, &got);
        return Ok(());
    }
    support::compare(f, &got).map_err(Failed::from)
}
