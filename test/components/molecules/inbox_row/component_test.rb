require "test_helper"

class Molecules::InboxRow::ComponentTest < ViewComponent::TestCase
  setup { @sample = Inbox::Sample.new }

  test "renders an awaiting run with chip, cost and a link when href is given" do
    row = render_inline(Molecules::InboxRow::Component.new(item: @sample.decisions.first, href: "/?item=run_31"))

    assert_equal 1, row.css("a[href='/?item=run_31']").size
    assert_includes row.text, "Configurable push mirror to GitHub"
    assert_includes row.text, "Run #31 · 6 archivos · 14 comandos"
    assert_includes row.text, "$1.84"
    assert_equal 1, row.css("span.bg-sun-400").size
  end

  test "marks the selected row" do
    row = render_inline(Molecules::InboxRow::Component.new(item: @sample.decisions.first, href: "/x", selected: true))

    assert_equal 1, row.css("div.bg-sun-100").size
    assert_equal "true", row.css("a").attr("aria-current").value
  end

  test "conflicting pull request shows the conflicting file" do
    row = render_inline(Molecules::InboxRow::Component.new(item: @sample.decisions.last))

    assert_includes row.text, "config/routes.rb"
    assert_empty row.css("a")
  end

  test "running run shows its command in monospace and the elapsed time" do
    row = render_inline(Molecules::InboxRow::Component.new(item: @sample.working.first))

    assert_includes row.css("p.font-mono").text, "bin/rails test"
    assert_includes row.text, "4m 18s"
  end

  test "unassigned issue offers to assign an agent and has no cost" do
    row = render_inline(Molecules::InboxRow::Component.new(item: @sample.issues.second))

    assert_equal 1, row.css("button[type=button]").size
    assert_not_includes row.text, "$"
  end

  test "assigned issue shows its run instead of a button" do
    row = render_inline(Molecules::InboxRow::Component.new(item: @sample.issues.first))

    assert_empty row.css("button")
    assert_includes row.text, "Run #32"
  end
end
