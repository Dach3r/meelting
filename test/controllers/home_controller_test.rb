require "test_helper"

class HomeControllerTest < ActionDispatch::IntegrationTest
  test "requires authentication through the root constraint" do
    get root_path

    assert_equal "landing", @controller.controller_name
  end

  test "shows create repository instead of the inbox without repositories" do
    users(:one).repositories.destroy_all
    sign_in_as users(:one)
    get root_path

    assert_response :success
    assert_select "h1", "Crea tu primer repositorio"
    assert_select "a[href=?]", new_repository_path, "Crear repositorio"
    assert_select "section#inbox", count: 0
    assert_select "section#in-progress", count: 0
  end

  test "shows the inbox with a repository that was never pushed" do
    sign_in_as users(:one)
    get root_path

    assert_select "section#inbox"
    assert_select "section#no_repositories", count: 0
    assert_select "[role=alert]", count: 0
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

  test "previews the home without repositories in development" do
    in_development do
      sign_in_as users(:one)
      get root_path(preview: "no_repositories")
    end

    assert_select "h1", "Crea tu primer repositorio"
    assert_select "section#inbox", count: 0
    assert_select "section#in-progress", count: 0
  end

  test "ignores the preview param outside development" do
    sign_in_as users(:one)
    get root_path(preview: "inbox")

    assert_select "section#inbox li", count: 0
    assert_select "*", text: /Datos de ejemplo/, count: 0
  end

  test "navbar shows the user menu" do
    sign_in_as users(:one)
    get repositories_path

    assert_select "nav summary", /A\s+Ada Lovelace/
  end
end
