require "test_helper"

class Molecules::RailItem::ComponentTest < ViewComponent::TestCase
  test "renders an icon-only link labelled for assistive tech" do
    link = render_inline(Molecules::RailItem::Component.new(label: "Inbox", href: "/", icon: :inbox)).css("a[href='/']").first

    assert_equal "Inbox", link["aria-label"]
    assert_equal 1, link.css("svg").size
    assert_nil link["aria-current"]
    assert_includes link["class"], "text-forest-muted"
  end

  test "marks the current page" do
    link = render_inline(Molecules::RailItem::Component.new(label: "Inbox", href: "/", icon: :inbox, current: true)).css("a").first

    assert_equal "page", link["aria-current"]
    assert_includes link["class"], "bg-forest-active"
  end

  test "renders a badge" do
    node = render_inline(Molecules::RailItem::Component.new(label: "Inbox", href: "/", icon: :inbox)) do |item|
      item.with_badge { "<span class='badge'>3</span>".html_safe }
    end

    assert_equal "3", node.css("a span.badge").text
  end
end
