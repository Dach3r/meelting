require "test_helper"

class Organisms::LandingHero::ComponentTest < ViewComponent::TestCase
  test "renders the headline" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_equal "Construye software junto a agentes de IA, con trazabilidad total.", hero.css("h1").text.squish
  end

  test "renders the three steps" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_equal 3, hero.css("ul li").size
    assert_equal %w[1 2 3], hero.css("ul li span").map { |badge| badge.text.strip }
    assert_equal %w[Recuerda Controla Rastrea], hero.css("ul li").map { |item| item.text.gsub(/\d/, "").strip }
  end
end
