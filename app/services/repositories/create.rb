module Repositories
  # Saves the repository and creates its bare repo on disk. With a license, the default branch
  # gets one "Initial commit" holding a single LICENSE file, written with git plumbing. The repo
  # is built in a staging directory and moved into place, so a failure leaves neither a row nor
  # a directory behind.
  class Create
    LICENSE_FILE = "LICENSE"
    INITIAL_COMMIT_MESSAGE = "Initial commit"

    def self.call(repository, git_class: Repository::Git)
      new(repository, git_class).call
    end

    def initialize(repository, git_class)
      @repository = repository
      @git_class = git_class
    end

    def call
      return false unless repository.valid?

      Repository.transaction do
        repository.save!
        build_on_disk
      end
      true
    rescue Repository::Git::Error, SystemCallError => error
      Rails.error.report(error, handled: true)
      repository.errors.add(:base, :git_failed)
      false
    end

    private
    attr_reader :repository, :git_class

    def build_on_disk
      final = git_class.new(repository.git_path)
      staging = git_class.new(repository.git_path.dirname.join(".staging-#{repository.id}-#{SecureRandom.hex(4)}.git"))
      FileUtils.mkdir_p(final.path.dirname)

      final.with_lock do
        staging.init_bare(default_branch: repository.default_branch)
        commit_license(staging) if license
        File.rename(staging.path, final.path)
      end
    rescue StandardError
      FileUtils.rm_f("#{final.path}.lock") if final
      raise
    ensure
      FileUtils.rm_rf(staging.path) if staging
    end

    def commit_license(git)
      blob = git.hash_object(license.render(user: user))
      tree = git.mktree([Repository::Git::Entry.new(mode: "100644", type: "blob", sha: blob, name: LICENSE_FILE)])
      commit = git.commit_tree(tree, message: INITIAL_COMMIT_MESSAGE, author: author, committer: committer)
      git.create_ref("refs/heads/#{repository.default_branch}", commit)
    end

    def license
      repository.license
    end

    def user
      repository.user
    end

    def author
      Repository::Git::Identity.new(name: user.full_name, email: user.email_address)
    end

    def committer
      Repository::Git::Identity.new(**Rails.configuration.x.git_committer)
    end
  end
end
