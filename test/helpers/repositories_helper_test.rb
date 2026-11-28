require "test_helper"

class RepositoriesHelperTest < ActionView::TestCase
  test "starts on the first step without errors" do
    assert_equal 1, repository_wizard_step(Repository.new)
  end

  test "opens the step that holds the first error" do
    repository = Repository.new
    repository.errors.add(:license, :inclusion)
    assert_equal 2, repository_wizard_step(repository)

    repository.errors.clear
    repository.errors.add(:default_branch, :blank)
    assert_equal 3, repository_wizard_step(repository)
  end
end
