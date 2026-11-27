require "test_helper"

class Molecules::CountPill::ComponentTest < ViewComponent::TestCase
  test "renders the count and label inside a link" do
    pill = render_inline(Molecules::CountPill::Component.new(count: 2, label: "waiting", href: "/x", tone: :wait)).css("a[href='/x']")

    assert_includes pill.text, "2"
    assert_includes pill.text, "waiting"
    assert_includes pill.attr("class").value, "bg-sun-400"
  end

  test "neutral tone is the default" do
    pill = render_inline(Molecules::CountPill::Component.new(count: 0, label: "running", href: "/y")).css("a")

    assert_includes pill.attr("class").value, "bg-graphite-50"
  end
end
