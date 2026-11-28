require "test_helper"

class Molecules::Brand::ComponentTest < ViewComponent::TestCase
  test "links to root with the brand name and a lime dot" do
    brand = render_inline(Molecules::Brand::Component.new)

    assert_equal ApplicationHelper::CONFIG[:name], brand.css("a[href='/']").text.strip
    assert_includes brand.css("a span[aria-hidden='true']").attr("class").value, "bg-lime"
    assert_equal 0, brand.css("a svg").size
  end

  test "dark tone uses ink text" do
    brand = render_inline(Molecules::Brand::Component.new(tone: :dark))

    assert_includes brand.css("a").attr("class").value, "text-ink"
  end

  test "light tone uses white text for the forest background" do
    brand = render_inline(Molecules::Brand::Component.new(tone: :light))

    assert_includes brand.css("a").attr("class").value, "text-white"
  end
end
