require "test_helper"

class Organisms::LandingHero::ComponentTest < ViewComponent::TestCase
  test "renders the headline" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_includes hero.css("h1").text, "La memoria de tu proyecto"
  end

  test "renders the three steps with title and text" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_equal 3, hero.css("ul li").size
    assert_equal %w[Recuerda Controla Rastrea], hero.css("ul li h2").map(&:text)
  end
end
