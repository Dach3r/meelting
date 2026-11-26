require "test_helper"

class RepositoriesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:one)
    sign_in_as @user
  end

  def valid_params(overrides = {})
    { repository: { name: "Docs", slug: "docs", description: "Notes", visibility: "public", license_id: licenses(:mit).id, default_branch: "main" }.merge(overrides) }
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

  test "new renders form" do
    get new_repository_path
    assert_response :success
  end

  test "create stores the repository for the current user" do
    assert_difference "@user.repositories.count", 1 do
      post repositories_path, params: valid_params
    end
    assert_redirected_to repositories_path
    repository = @user.repositories.find_by!(slug: "docs")
    assert repository.visibility_public?
    assert_equal licenses(:mit), repository.license
  end

  test "create without name fails" do
    assert_no_difference "Repository.count" do
      post repositories_path, params: valid_params(name: "")
    end
    assert_response :unprocessable_entity
  end

  test "create with duplicate slug fails" do
    assert_no_difference "Repository.count" do
      post repositories_path, params: valid_params(slug: repositories(:meelting).slug)
    end
    assert_response :unprocessable_entity
  end

  test "create with inactive license fails" do
    assert_no_difference "Repository.count" do
      post repositories_path, params: valid_params(license_id: licenses(:retired).id)
    end
    assert_response :unprocessable_entity
  end
end
