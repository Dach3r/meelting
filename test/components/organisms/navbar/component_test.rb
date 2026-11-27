require "test_helper"

class Organisms::Navbar::ComponentTest < ViewComponent::TestCase
  test "renders only the sign in link when unauthenticated" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: false))

    assert_equal 1, nav.css("a[href='/session/new']").size
    assert_empty nav.css("a[href='/registration/new']")
    assert_empty nav.css("form[action='/session']")
    assert_empty nav.css("a[href='/repositories']")
  end

  test "renders navigation and sign out button when authenticated" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: true, user: users(:one)))

    assert_equal 1, nav.css("a[href='/repositories']").size
    assert_equal 1, nav.css("a[href='/access_tokens']").size
    assert_equal 1, nav.css("form[action='/session'] button").size
    assert_empty nav.css("a[href='/session/new']")
  end

  test "brand links to root in both states" do
    [true, false].each do |authenticated|
      nav = render_inline(Organisms::Navbar::Component.new(authenticated: authenticated, user: (users(:one) if authenticated)))

      assert_equal ApplicationHelper::CONFIG[:name], nav.css("a[href='/']").text.strip
    end
  end

  test "stays fixed at the top in both states" do
    [true, false].each do |authenticated|
      header = render_inline(Organisms::Navbar::Component.new(authenticated: authenticated, user: (users(:one) if authenticated))).css("header")

      assert_includes header.attr("class").value, "sticky"
      assert_includes header.attr("class").value, "top-0"
      assert_includes header.attr("class").value, "bg-white"
    end
  end

  test "sign in link has the same height as the signed in controls" do
    link = render_inline(Organisms::Navbar::Component.new(authenticated: false)).css("a[href='/session/new']")

    assert_includes link.attr("class").value, "h-14"
  end
end
