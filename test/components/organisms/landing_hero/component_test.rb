require "test_helper"

class Organisms::LandingHero::ComponentTest < ViewComponent::TestCase
  test "renders the headline and the guest calls to action" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_includes hero.css("h1").text, "Un amarre seguro"
    assert_equal 2, hero.css("a[href='/registration/new']").size
    assert_operator hero.css("a[href='/session/new']").size, :>=, 1
  end

  test "offers a copy button for the self-host command" do
    hero = render_inline(Organisms::LandingHero::Component.new)
    chip = hero.css("[data-controller='clipboard']")

    assert_equal "docker compose up", chip.attr("data-clipboard-text-value").value
    assert_equal 1, chip.css("button[data-action='clipboard#copy']").size
  end

  test "decorative agent chips are hidden from assistive tech" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_equal 1, hero.css("div[aria-hidden='true']").size
    assert_equal 4, hero.css("div[aria-hidden='true'] > span").size
  end
end
