require "test_helper"

class RepositoriesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:one)
    sign_in_as @user
  end

  def valid_params(overrides = {})
    { repository: { name: "Docs", description: "Notes", visibility: "public", license_id: licenses(:mit).id, default_branch: "main" }.merge(overrides) }
  end

  test "requires authentication" do
    sign_out
    get new_repository_path
    assert_redirected_to new_session_path
  end

  test "index lists only the current user's repositories" do
    get repositories_path
    assert_response :success
    assert_select "##{dom_id(repositories(:meelting))}"
    assert_select "##{dom_id(repositories(:public_notes))}", count: 0
  end

  test "new renders the fields in order with private selected" do
    get new_repository_path
    assert_response :success
    fields = css_select("form input:not([type=hidden]), form select, form textarea").map { |field| field["id"] }
    assert_equal %w[repository_name repository_description repository_visibility_private repository_visibility_public repository_default_branch repository_license_id], fields
    assert_select "#repository_name[autofocus]"
    assert_select "#repository_description[rows='2']:not([required])"
    assert_select "label[for=repository_description]", "Descripción (opcional)"
    assert_select "#repository_visibility_private[checked]"
    assert_select "#repository_visibility_public[checked]", 0
  end

  test "create stores the repository for the current user" do
    assert_difference "@user.repositories.count", 1 do
      post repositories_path, params: valid_params
    end
    assert_redirected_to repository_section_path("docs", :code)
    repository = @user.repositories.find_by!(slug: "docs")
    assert repository.visibility_public?
    assert_equal licenses(:mit), repository.license
  end

  test "create derives the slug from the name and ignores a submitted one" do
    post repositories_path, params: valid_params(name: "Los de la culpa 2026", slug: "custom")
    assert_redirected_to repository_section_path("los-de-la-culpa-2026", :code)
  end

  test "new has no slug input" do
    get new_repository_path
    assert_select "#repository_slug", 0
  end

  test "create without name fails" do
    assert_no_difference "Repository.count" do
      post repositories_path, params: valid_params(name: "")
    end
    assert_response :unprocessable_entity
  end

  test "create with duplicate slug fails" do
    assert_no_difference "Repository.count" do
      post repositories_path, params: valid_params(name: repositories(:meelting).name)
    end
    assert_response :unprocessable_entity
  end

  test "create failure re-renders the form with errors" do
    post repositories_path, params: valid_params(default_branch: "")
    assert_response :unprocessable_entity
    assert_select "#error_explanation"
    assert_select "#repository_default_branch"
  end

  test "create with inactive license fails" do
    assert_no_difference "Repository.count" do
      post repositories_path, params: valid_params(license_id: licenses(:retired).id)
    end
    assert_response :unprocessable_entity
  end
end
