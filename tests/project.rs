use hecks_gherkin::project;

const SENTENCE: &str = include_str!("fixtures/minimal/sentence_form.bluebook");
const EXPECTED: &str = include_str!("fixtures/minimal/expected.bluebook");

#[test]
fn minimal_domain_projects_to_bluebook() {
    assert_eq!(project(SENTENCE).expect("the minimal domain projects"), EXPECTED);
}

#[test]
fn an_unknown_sentence_is_refused_with_its_column() {
    let source = "Domain: Library\n  Members may do something odd\n";
    let refusal = project(source).expect_err("an unknown sentence is refused");
    assert_eq!((refusal.line, refusal.column), (2, 3));
}

#[test]
fn a_file_without_a_domain_is_refused() {
    let refusal = project("Aggregate: Book\n").expect_err("no domain");
    assert_eq!(refusal.line, 1);
}
