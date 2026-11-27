require "test_helper"

class Molecules::Brand::ComponentTest < ViewComponent::TestCase
  test "links to root with the brand name and a colored dot" do
    brand = render_inline(Molecules::Brand::Component.new)

    assert_equal ApplicationHelper::CONFIG[:name], brand.css("a[href='/']").text.strip
    assert_equal 1, brand.css("a span[aria-hidden='true']").size
    assert_equal 0, brand.css("a svg").size
  end

  test "light tone uses the brand green dot" do
    brand = render_inline(Molecules::Brand::Component.new)

    assert_includes brand.css("a span").attr("class").value, "bg-brand-500"
  end

  test "deep tone uses the light text color and the lime dot" do
    brand = render_inline(Molecules::Brand::Component.new(tone: :deep))

    assert_includes brand.css("a").attr("class").value, "text-white"
    assert_includes brand.css("a span").attr("class").value, "bg-sun-400"
  end
end
