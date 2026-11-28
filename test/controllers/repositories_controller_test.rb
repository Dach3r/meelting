require "test_helper"

class RepositoriesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = users(:one)
    sign_in_as @user
    @original_git_root = Rails.configuration.x.git_root
    Rails.configuration.x.git_root = Pathname(Dir.mktmpdir)
  end

  teardown do
    FileUtils.rm_rf(Rails.configuration.x.git_root)
    Rails.configuration.x.git_root = @original_git_root
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
    get user_repositories_path("ada")
    assert_response :success
    assert_select "##{dom_id(repositories(:meelting))}"
    assert_select "##{dom_id(repositories(:public_notes))}", count: 0
  end

  test "index of another user is not found" do
    get user_repositories_path("alan")
    assert_response :not_found
  end

  test "new renders the fields in order with private selected" do
    get new_repository_path
    assert_response :success
    fields = css_select("form input:not([type=hidden]), form select, form textarea").map { |field| field["name"] }.uniq
    assert_equal %w[repository[name] repository[description] repository[visibility] repository[default_branch] repository[license_id]], fields
    assert_select "#repository_name[autofocus]"
    assert_select "input#repository_description[type=text]:not([required])"
    assert_select "label[for=repository_description]", "Descripción"
    assert_select "[role=radiogroup][aria-labelledby=repository_visibility_label]"
    assert_select "a[href=?]", user_repositories_path(@user.username), "Cancelar"
    assert_select "#repository_visibility_private[checked]"
    assert_select "#repository_visibility_public[checked]", 0
  end

  test "create stores the repository for the current user" do
    assert_difference "@user.repositories.count", 1 do
      post repositories_path, params: valid_params
    end
    assert_redirected_to repository_path("ada", "docs")
    repository = @user.repositories.find_by!(slug: "docs")
    assert repository.visibility_public?
    assert_equal licenses(:mit), repository.license
    assert repository.initialized?
    assert_equal ["LICENSE"], repository.git.ls_tree(repository.head_sha).map(&:name)
  end

  test "create without a license leaves an empty bare repository" do
    post repositories_path, params: valid_params(license_id: "")

    repository = @user.repositories.find_by!(slug: "docs")
    assert_nil repository.license
    assert repository.git_path.directory?
    assert_not repository.initialized?
  end

  test "new offers no license first and the selectable licenses in order" do
    License.sync_catalog
    get new_repository_path

    options = css_select("input[type=radio][name='repository[license_id]']")
    assert_equal ["", *License::SELECTABLE_KEYS.map { |key_| License.find_by!(key: key_).id.to_s }], options.map { |option_| option_["value"] }
    assert_select "#repository_license_id_[checked]"
    assert_select "[data-select-menu-target=label]", "Sin licencia"
    assert_select "#repository_license_label", "Licencia"
    assert_select "fieldset[aria-labelledby=repository_license_label]"
  end

  test "create derives the slug from the name and ignores a submitted one" do
    post repositories_path, params: valid_params(name: "Los de la culpa 2026", slug: "custom")
    assert_redirected_to repository_path("ada", "los-de-la-culpa-2026")
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
