require "test_helper"

class Molecules::Brand::ComponentTest < ViewComponent::TestCase
  test "links to root labelled with the brand name in braces and a lime i dot" do
    brand = render_inline(Molecules::Brand::Component.new)

    assert_equal ApplicationHelper::APP_NAME, brand.css("a[href='/']").attr("aria-label").value
    assert_equal "{Meeltıng}", brand.css("a").text.gsub(/\s/, "")
    assert_equal 1, brand.css("a span.bg-lime").size
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
