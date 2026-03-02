use std::path::Path;

use rubyfast::analyzer::analyze_file;
use rubyfast::config::Config;
use rubyfast::offense::OffenseKind;

fn analyze(fixture: &str) -> Vec<OffenseKind> {
    let path = Path::new("tests/fixtures").join(fixture);
    let config = Config::default();
    let result = analyze_file(&path, &config).unwrap();
    result.offenses.iter().map(|o| o.kind).collect()
}

fn count_kind(kinds: &[OffenseKind], target: OffenseKind) -> usize {
    kinds.iter().filter(|&&k| k == target).count()
}

#[test]
fn shuffle_first() {
    let kinds = analyze("01_shuffle_first.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::ShuffleFirstVsSample), 3);
}

#[test]
fn select_first() {
    let kinds = analyze("02_select_first.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::SelectFirstVsDetect), 3);
}

#[test]
fn select_last() {
    let kinds = analyze("03_select_last.rb");
    assert_eq!(
        count_kind(&kinds, OffenseKind::SelectLastVsReverseDetect),
        1
    );
}

#[test]
fn reverse_each() {
    let kinds = analyze("04_reverse_each.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::ReverseEachVsReverseEach), 2);
}

#[test]
fn keys_each() {
    let kinds = analyze("05_keys_each.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::KeysEachVsEachKey), 3);
}

#[test]
fn map_flatten() {
    let kinds = analyze("06_map_flatten.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::MapFlattenVsFlatMap), 2);
}

#[test]
fn gsub_vs_tr() {
    let kinds = analyze("07_gsub_vs_tr.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::GsubVsTr), 2);
}

#[test]
fn sort_vs_sort_by() {
    let kinds = analyze("08_sort_vs_sort_by.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::SortVsSortBy), 1);
}

#[test]
fn fetch_with_argument() {
    let kinds = analyze("09_fetch_with_argument.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::FetchWithArgumentVsBlock), 1);
}

#[test]
fn hash_merge_bang() {
    let kinds = analyze("10_hash_merge_bang.rb");
    assert!(count_kind(&kinds, OffenseKind::HashMergeBangVsHashBrackets) >= 2);
}

#[test]
fn block_vs_symbol_to_proc() {
    let kinds = analyze("11_block_vs_symbol_to_proc.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::BlockVsSymbolToProc), 3);
}

#[test]
fn each_with_index() {
    let kinds = analyze("12_each_with_index.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::EachWithIndexVsWhile), 1);
}

#[test]
fn include_vs_cover() {
    let kinds = analyze("13_include_vs_cover.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::IncludeVsCoverOnRange), 2);
}

#[test]
fn module_eval() {
    let kinds = analyze("14_module_eval.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::ModuleEval), 1);
}

#[test]
fn rescue_vs_respond_to() {
    let kinds = analyze("15_rescue_vs_respond_to.rb");
    // 2 rescues with NoMethodError, 1 without
    assert_eq!(count_kind(&kinds, OffenseKind::RescueVsRespondTo), 2);
}

#[test]
fn proc_call_vs_yield() {
    let kinds = analyze("16_proc_call_vs_yield.rb");
    // 2 methods with &block that call block.call, 1 without &block
    assert_eq!(count_kind(&kinds, OffenseKind::ProcCallVsYield), 2);
}

#[test]
fn getter_vs_attr_reader() {
    let kinds = analyze("17_getter_vs_attr_reader.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::GetterVsAttrReader), 2);
}

#[test]
fn setter_vs_attr_writer() {
    let kinds = analyze("18_setter_vs_attr_writer.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::SetterVsAttrWriter), 2);
}

#[test]
fn for_loop() {
    let kinds = analyze("19_for_loop.rb");
    assert_eq!(count_kind(&kinds, OffenseKind::ForLoopVsEach), 1);
}

#[test]
fn inline_disable() {
    let kinds = analyze("20_inline_disable.rb");
    // Lines 7, 13, 24 should report offenses (3 shuffle_first + 1 for_loop)
    assert_eq!(count_kind(&kinds, OffenseKind::ShuffleFirstVsSample), 2);
    assert_eq!(count_kind(&kinds, OffenseKind::ForLoopVsEach), 1);
}

#[test]
fn clean_file_has_no_offenses() {
    let kinds = analyze("clean.rb");
    assert!(kinds.is_empty(), "Expected no offenses, got: {:?}", kinds);
}
