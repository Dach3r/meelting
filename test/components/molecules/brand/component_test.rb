require "test_helper"

class Molecules::Brand::ComponentTest < ViewComponent::TestCase
  test "links to root with the brand name and the braces mark" do
    brand = render_inline(Molecules::Brand::Component.new)

    assert_equal ApplicationHelper::APP_NAME, brand.css("a[href='/']").text.strip
    assert_equal 1, brand.css("a svg[aria-hidden='true']").size
    assert_includes brand.css("a svg circle").attr("class").value, "fill-lime"
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
