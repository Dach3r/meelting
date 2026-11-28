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

  test "new renders every field in a single form" do
    get new_repository_path
    assert_response :success
    assert_select "form[data-controller=repository-form]" do
      assert_select "#repository_name"
      assert_select "#repository_default_branch"
      assert_select "#repository_description"
      assert_select "input[name='repository[visibility]']", 2
      assert_select "#repository_license_id"
    end
    assert_select "[hidden] input, [hidden] select, [hidden] textarea", 0
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

  test "new passes the url template and taken slugs to the form" do
    get new_repository_path
    assert_select "form[data-repository-form-url-template-value=?]", "http://www.example.com/repositories/__slug__/code"
    assert_select "form[data-repository-form-taken-slugs-value=?]", [repositories(:meelting).slug].to_json
    assert_select "[data-repository-form-target=url][hidden]"
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
