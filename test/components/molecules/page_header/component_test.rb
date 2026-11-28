require "test_helper"

class Molecules::PageHeader::ComponentTest < ViewComponent::TestCase
  test "renders the title, subtitle and an initial avatar" do
    node = render_inline(Molecules::PageHeader::Component.new(title: "Configuración", subtitle: "Ada Lovelace · ada", initial: "A"))

    assert_equal "Configuración", node.css("h1").text
    assert_equal "Ada Lovelace · ada", node.css("p").text
    assert_equal "A", node.css("span[aria-hidden]").text.strip
  end

  test "renders an icon avatar at another heading level and an aside" do
    node = render_inline(Molecules::PageHeader::Component.new(title: "Inbox", icon: :inbox, level: :h2)) do |header|
      header.with_aside { "chip" }
    end

    assert_equal "Inbox", node.css("h2").text
    assert_equal 1, node.css("span[aria-hidden] svg").size
    assert_includes node.text, "chip"
  end

  test "omits the avatar and subtitle when not given" do
    node = render_inline(Molecules::PageHeader::Component.new(title: "Runs"))

    assert_empty node.css("span[aria-hidden]")
    assert_empty node.css("p")
  end
end
