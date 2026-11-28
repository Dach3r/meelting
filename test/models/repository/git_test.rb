require "test_helper"

class Repository::GitTest < ActiveSupport::TestCase
  test "builds a commit with plumbing on a bare repository" do
    Dir.mktmpdir do |dir_|
      git = Repository::Git.new(Pathname(dir_).join("x.git"))
      git.init_bare(default_branch: "trunk")

      blob = git.hash_object("hello\n")
      tree = git.mktree([Repository::Git::Entry.new(mode: "100644", type: "blob", sha: blob, name: "a file.txt")])
      person = Repository::Git::Identity.new(name: "Ada", email: "ada@example.com")
      commit = git.commit_tree(tree, message: "Initial commit", author: person, committer: person)
      git.create_ref("refs/heads/trunk", commit)

      assert_equal commit, git.resolve("trunk")
      assert_equal ["a file.txt"], git.ls_tree(commit).map(&:name)
      assert_equal "hello\n", git.cat_blob(blob)
    end
  end

  test "resolve returns nil for a missing branch" do
    Dir.mktmpdir do |dir_|
      git = Repository::Git.new(Pathname(dir_).join("x.git"))
      git.init_bare(default_branch: "main")

      assert_nil git.resolve("main")
    end
  end

  test "create_ref refuses to overwrite an existing branch" do
    Dir.mktmpdir do |dir_|
      git = Repository::Git.new(Pathname(dir_).join("x.git"))
      git.init_bare(default_branch: "main")
      tree = git.mktree([])
      person = Repository::Git::Identity.new(name: "Ada", email: "ada@example.com")
      first = git.commit_tree(tree, message: "one", author: person, committer: person)
      git.create_ref("refs/heads/main", first)

      assert_raises(Repository::Git::Error) { git.create_ref("refs/heads/main", first) }
    end
  end
end
