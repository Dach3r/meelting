require "test_helper"

class Organisms::Navbar::ComponentTest < ViewComponent::TestCase
  test "renders sign in and sign up links when unauthenticated" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: false))

    assert_equal 1, nav.css("a[href='/session/new']").size
    assert_equal 1, nav.css("a[href='/registration/new']").size
    assert_empty nav.css("form[action='/session']")
    assert_empty nav.css("a[href='/repositories']")
  end

  test "renders navigation and sign out button when authenticated" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: true))

    assert_equal 1, nav.css("a[href='/repositories']").size
    assert_equal 1, nav.css("a[href='/access_tokens']").size
    assert_equal 1, nav.css("form[action='/session'] button").size
    assert_empty nav.css("a[href='/session/new']")
  end

  test "brand links to root in both states" do
    [true, false].each do |authenticated|
      nav = render_inline(Organisms::Navbar::Component.new(authenticated: authenticated))

      assert_equal "Berth", nav.css("a[href='/']").text.strip
    end
  end

  test "floating variant adds section anchors for guests" do
    nav = render_inline(Organisms::Navbar::Component.new(authenticated: false, variant: :floating))

    assert_equal 1, nav.css("a[href='#features']").size
    assert_equal 1, nav.css("a[href='#memory']").size
    assert_equal 1, nav.css("a[href='#self-host']").size
    assert_equal 1, nav.css("a[href='/registration/new']").size
  end
end
