require "test_helper"

class Molecules::Brand::ComponentTest < ViewComponent::TestCase
  test "links to root with the brand name and mark" do
    brand = render_inline(Molecules::Brand::Component.new)

    assert_equal "Berth", brand.css("a[href='/']").text.strip
    assert_equal 1, brand.css("a svg").size
  end

  test "deep tone uses the light text color" do
    brand = render_inline(Molecules::Brand::Component.new(tone: :deep))

    assert_includes brand.css("a").attr("class").value, "text-deep-ink"
  end
end
