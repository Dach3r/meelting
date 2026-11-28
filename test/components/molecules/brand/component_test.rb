require "test_helper"

class Molecules::Brand::ComponentTest < ViewComponent::TestCase
  test "links to root labelled with the brand name and shows it lowercase in braces" do
    brand = render_inline(Molecules::Brand::Component.new)

    assert_equal ApplicationHelper::APP_NAME, brand.css("a[href='/']").attr("aria-label").value
    assert_equal "{meelting}", brand.css("a").text.gsub(/\s/, "")
  end

  test "dark tone uses ink text, forest braces and a deep lime i dot" do
    brand = render_inline(Molecules::Brand::Component.new(tone: :dark))

    assert_includes brand.css("a").attr("class").value, "text-ink"
    assert_equal 2, brand.css("span.text-forest").size
    assert_equal 1, brand.css("span.bg-clip-text.from-lime-deep").size
  end

  test "light tone uses white text with lime braces and i dot for the forest background" do
    brand = render_inline(Molecules::Brand::Component.new(tone: :light))

    assert_includes brand.css("a").attr("class").value, "text-white"
    assert_equal 2, brand.css("span.text-lime").size
    assert_equal 1, brand.css("span.bg-clip-text.from-lime").size
  end

  test "compact renders braces around a white dot without the name" do
    brand = render_inline(Molecules::Brand::Component.new(tone: :light, compact: true))

    assert_equal ApplicationHelper::APP_NAME, brand.css("a[href='/']").attr("aria-label").value
    assert_equal "{}", brand.css("a").text.gsub(/\s/, "")
    assert_equal 1, brand.css("a span.rounded-full.bg-white").size
  end
end
