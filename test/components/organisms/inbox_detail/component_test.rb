require "test_helper"

class Organisms::InboxDetail::ComponentTest < ViewComponent::TestCase
  setup { @sample = Inbox::Sample.new }

  test "run detail shows stats, steps, memory and exactly one primary button" do
    detail = render_inline(Organisms::InboxDetail::Component.new(item: @sample.decisions.first))

    assert_includes detail.text, "Esperando aprobación"
    assert_includes detail.text, "+214"
    assert_includes detail.text, "14 · 1 falló, reintentado"
    assert_includes detail.text, "$1.84 · 11m 42s"
    assert_equal 4, detail.css("ol li").size
    assert_equal 3, detail.css("ul li").size
    assert_equal 1, detail.css("button.bg-sun-400").size
    assert_includes detail.css("button.bg-sun-400").text, "Aprobar y abrir PR"
    assert_includes detail.text, "No limita ni deshace"
  end

  test "run without retries does not mention a failure" do
    detail = render_inline(Organisms::InboxDetail::Component.new(item: @sample.decisions.second))

    assert_not_includes detail.text, "reintentado"
  end

  test "pull request detail shows tests and a review button without approval buttons" do
    detail = render_inline(Organisms::InboxDetail::Component.new(item: @sample.decisions.third))

    assert_includes detail.text, "214 tests, 0 fallas"
    assert_includes detail.text, "PR #8 · run #28"
    assert_equal 1, detail.css("button").size
    assert_empty detail.css("ol")
  end

  test "conflicting pull request is flagged" do
    detail = render_inline(Organisms::InboxDetail::Component.new(item: @sample.decisions.last))

    assert_includes detail.text, "Conflictos con main"
    assert_equal 1, detail.css("span.bg-coral-100").size
  end
end
