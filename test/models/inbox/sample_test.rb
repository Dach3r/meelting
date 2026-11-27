require "test_helper"

class Inbox::SampleTest < ActiveSupport::TestCase
  test "selects the first decision by default" do
    assert_equal "run_31", Inbox::Sample.new.selected.id
  end

  test "selects the decision matching the given id" do
    assert_equal "pr_8", Inbox::Sample.new("pr_8").selected.id
  end

  test "falls back to the first decision for an unknown id" do
    assert_equal "run_31", Inbox::Sample.new("nope").selected.id
  end

  test "items expose their kind and state" do
    sample = Inbox::Sample.new

    assert sample.decisions.first.run?
    assert sample.decisions.first.awaiting_approval?
    assert sample.decisions.third.pull_request?
    assert sample.working.first.running?
    assert sample.issues.first.issue?
  end

  test "detail raises for a missing key" do
    assert_raises(KeyError) { Inbox::Sample.new.issues.second.detail(:run_number) }
  end
end
