require "test_helper"

class Molecules::Tray::ComponentTest < ViewComponent::TestCase
  test "renders the title, count and content" do
    tray = render_inline(Molecules::Tray::Component.new(title: "Open", count: 3, id: "open")) { "rows" }

    assert_includes tray.css("section#open h2").text, "Open"
    assert_includes tray.css("h2").text, "3"
    assert_includes tray.text, "rows"
  end

  test "omits the count when not given" do
    tray = render_inline(Molecules::Tray::Component.new(title: "Open")) { "rows" }

    assert_empty tray.css("h2 span")
  end
end
