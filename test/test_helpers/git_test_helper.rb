module GitTestHelper
  def with_git_root
    Dir.mktmpdir do |dir_|
      original = Rails.configuration.x.git_root
      Rails.configuration.x.git_root = Pathname(dir_)
      yield Pathname(dir_)
    ensure
      Rails.configuration.x.git_root = original
    end
  end

  def create_bare_repository(repository, files: {}, branch: repository.default_branch)
    Dir.mktmpdir do |work_|
      git = ->(*args_) { system("git", "-C", work_, *args_, exception: true, out: File::NULL, err: File::NULL) }
      git.call("init", "-q", "-b", branch)
      files.each do |path_, content_|
        FileUtils.mkdir_p(File.dirname(File.join(work_, path_)))
        File.write(File.join(work_, path_), content_)
      end
      git.call("add", "-A")
      git.call("-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-q", "--allow-empty", "-m", "Initial commit")
      system("git", "clone", "-q", "--bare", work_, repository.git_path.to_s, exception: true, out: File::NULL, err: File::NULL)
    end
  end
end
