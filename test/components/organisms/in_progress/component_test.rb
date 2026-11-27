require "test_helper"

class Organisms::InProgress::ComponentTest < ViewComponent::TestCase
  test "renders run, elapsed time, last event and the view all link" do
    node = render_inline(Organisms::InProgress::Component.new(runs: Workspace::PlaceholderData.runs(:inbox), preview: true))

    assert_includes node.text, "Merge commit with per-repository lock"
    assert_includes node.text, "4m 18s"
    assert_includes node.text, "bin/rails test"
    assert_equal "/runs", node.css("a").first["href"]
  end

  test "shows at most three runs" do
    run = Workspace::RunEntry.new(title: "r", last_event: "e", elapsed_seconds: 1)
    node = render_inline(Organisms::InProgress::Component.new(runs: [run] * 5, preview: false))

    assert_equal 3, node.css("li").size
  end
end
