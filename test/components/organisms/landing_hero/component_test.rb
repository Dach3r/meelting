require "test_helper"

class Organisms::LandingHero::ComponentTest < ViewComponent::TestCase
  test "renders the headline and the guest calls to action" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_includes hero.css("h1").text, "Un amarre seguro"
    assert_equal 1, hero.css("a[href='/registration/new']").size
  end

  test "renders the brand mark inside the rings" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_includes hero.css("[aria-hidden='true']").text, ApplicationHelper::CONFIG[:name].first
  end

  test "renders the three points and the sign in link" do
    hero = render_inline(Organisms::LandingHero::Component.new)

    assert_equal 3, hero.css("ul li").size
    assert_equal 1, hero.css("a[href='/session/new']").size
  end
end
