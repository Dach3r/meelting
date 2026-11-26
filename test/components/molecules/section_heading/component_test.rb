require "test_helper"

class Molecules::SectionHeading::ComponentTest < ViewComponent::TestCase
  test "renders label, title, dim title and aside" do
    html = render_inline(Molecules::SectionHeading::Component.new(label: "Why", title: "Main", title_dim: "Dim", aside: "Side"))

    assert_equal "Why", html.css("span").first.text.strip
    assert_includes html.css("h2").text, "Main"
    assert_equal "Dim", html.css("h2 span").text
    assert_equal "Side", html.css("p").text
  end

  test "omits dim title and aside when not given" do
    html = render_inline(Molecules::SectionHeading::Component.new(label: "Why", title: "Main"))

    assert_empty html.css("h2 span")
    assert_empty html.css("p")
  end
end
