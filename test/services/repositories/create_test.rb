require "test_helper"

class Repositories::CreateTest < ActiveSupport::TestCase
  class FailingGit < Repository::Git
    def commit_tree(*)
      raise Repository::Git::Error, "forced failure"
    end
  end

  setup do
    License.sync_catalog
    @user = users(:one)
  end

  def build(license_key: nil)
    @user.repositories.new(name: "Docs", license: license_key && License.find_by!(key: license_key))
  end

  def git_log(repository)
    out = IO.popen(["git", "--git-dir=#{repository.git_path}", "log", "--format=%an <%ae>|%cn <%ce>|%s", "main"], &:read)
    out.lines(chomp: true)
  end

  def files(repository)
    IO.popen(["git", "--git-dir=#{repository.git_path}", "ls-tree", "--name-only", "main"], &:read).lines(chomp: true)
  end

  def license_text(repository)
    IO.popen(["git", "--git-dir=#{repository.git_path}", "show", "main:LICENSE"], &:read)
  end

  test "without a license creates an empty bare repository" do
    with_git_root do
      repository = build

      assert Repositories::Create.call(repository)
      assert repository.persisted?
      assert repository.git_path.join("HEAD").file?
      assert_not repository.initialized?
    end
  end

  test "with mit creates one commit holding only the filled LICENSE" do
    with_git_root do
      repository = build(license_key: "mit")

      assert Repositories::Create.call(repository)

      committer = Rails.configuration.x.git_committer
      assert_equal ["Ada Lovelace <one@example.com>|#{committer[:name]} <#{committer[:email]}>|Initial commit"], git_log(repository)
      assert_equal ["LICENSE"], files(repository)
      text = license_text(repository)
      assert_includes text, "Copyright (c) #{Time.current.year} Ada Lovelace"
      assert_no_match(/\[year\]|\[fullname\]/, text)
    end
  end

  test "with apache keeps the appendix placeholders" do
    with_git_root do
      repository = build(license_key: "apache-2.0")

      assert Repositories::Create.call(repository)

      assert_includes license_text(repository), "Copyright [yyyy] [name of copyright owner]"
    end
  end

  test "a failure while committing leaves no repository behind" do
    with_git_root do |root_|
      repository = build(license_key: "mit")

      assert_no_difference "Repository.count" do
        assert_not Repositories::Create.call(repository, git_class: FailingGit)
      end
      assert repository.errors.added?(:base, :git_failed)
      assert_empty root_.children
    end
  end

  test "invalid repository is not created on disk" do
    with_git_root do |root_|
      assert_not Repositories::Create.call(@user.repositories.new(name: ""))
      assert_empty root_.children
    end
  end
end
