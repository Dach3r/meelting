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

  test "renders the preview label and a link when given" do
    tray = render_inline(Molecules::Tray::Component.new(title: "Open", preview: "Preview data", link: { label: "All", href: "/all" })) { "rows" }

    assert_includes tray.text, "Preview data"
    assert_equal "/all", tray.css("a").first["href"]
  end

  test "renders a list when asked to" do
    tray = render_inline(Molecules::Tray::Component.new(title: "Open", list: true)) { "<li>row</li>".html_safe }

    assert_equal 1, tray.css("ul li").size
  end
end
