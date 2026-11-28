require "test_helper"

class HomeControllerTest < ActionDispatch::IntegrationTest
  test "requires authentication through the root constraint" do
    get root_path

    assert_equal "landing", @controller.controller_name
  end

  test "shows the setup checklist instead of the inbox on a first visit" do
    sign_in_as users(:one)
    get root_path

    assert_response :success
    assert_select "h1", "Configura Meelting"
    assert_select "ol li", 4
    assert_select "ol li[aria-disabled=true]", /Disponible en Fase 2\/3/
    assert_select "section#inbox", count: 0
    assert_select "section#in-progress", count: 0
  end

  test "checks steps off with real data" do
    repositories(:meelting).touch_pushed
    sign_in_as users(:one)
    get root_path

    assert_select "ol li[aria-current=step]", /Agrega una model key/
    assert_select "ol li [aria-label=Hecho]", 1
    assert_select "[role=img]", text: "1 de 4"
  end

  test "shows the inbox instead of the checklist once onboarding is complete" do
    users(:one).complete_onboarding!
    sign_in_as users(:one)
    get root_path

    assert_select "h1", text: "Configura Meelting", count: 0
    assert_select "section#inbox"
    assert_select "[role=alert]", /model key/
  end

  test "previews the inbox with sorted items and in progress runs in development" do
    in_development do
      sign_in_as users(:one)
      get root_path(preview: "inbox")
    end

    assert_select "section#inbox li", 3
    assert_select "section#inbox li:nth-child(1)", /Falló/
    assert_select "section#inbox li:nth-child(3)", /Aprobar/
    assert_select "section#inbox li a", 3
    assert_select "section#inbox", /Datos de ejemplo/
    assert_select "section#in-progress li", 1
    assert_select "section#in-progress", /bin\/rails test/
    assert_select "[role=alert]", count: 0
  end

  test "previews the empty inbox with a banner and no in progress block in development" do
    in_development do
      sign_in_as users(:one)
      get root_path(preview: "empty")
    end

    assert_select "h3", "Nada te necesita"
    assert_select "a", "Asignar un issue"
    assert_select "section#in-progress", count: 0
    assert_select "[role=alert]", /mirror de GitHub de .* está fallando/
  end

  test "previews the checklist alone in development" do
    in_development do
      sign_in_as users(:one)
      get root_path(preview: "first_run")
    end

    assert_select "h1", "Configura Meelting"
    assert_select "section#inbox", count: 0
    assert_select "section#in-progress", count: 0
  end

  test "ignores the preview param outside development" do
    sign_in_as users(:one)
    get root_path(preview: "inbox")

    assert_select "h1", "Configura Meelting"
    assert_select "section#inbox", count: 0
    assert_select "*", text: /Datos de ejemplo/, count: 0
  end

  test "navbar shows the user menu" do
    sign_in_as users(:one)
    get repositories_path

    assert_select "nav summary", /A\s+Ada Lovelace/
  end
end
