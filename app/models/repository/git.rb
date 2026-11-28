require "open3"

class Repository
  # Thin wrapper over the git CLI for one bare repository. Arguments are always passed
  # as an array, never through a shell.
  class Git
    Error = Class.new(StandardError)
    Entry = Data.define(:mode, :type, :sha, :name)
    Identity = Data.define(:name, :email)

    attr_reader :path

    def initialize(path)
      @path = Pathname(path)
    end

    def init_bare(default_branch:)
      git("init", "--quiet", "--bare", "--initial-branch=#{default_branch}", path.to_s, git_dir: false)
    end

    def with_lock
      File.open("#{path}.lock", File::RDWR | File::CREAT, 0o644) do |file_|
        file_.flock(File::LOCK_EX)
        yield
      end
    end

    def hash_object(content)
      git("hash-object", "-w", "--stdin", stdin: content).strip
    end

    def mktree(entries)
      git("mktree", "-z", stdin: entries.map { |entry_| "#{entry_.mode} #{entry_.type} #{entry_.sha}\t#{entry_.name}\0" }.join).strip
    end

    def commit_tree(tree, message:, author:, committer:, parents: [])
      env = {
        "GIT_AUTHOR_NAME" => author.name, "GIT_AUTHOR_EMAIL" => author.email,
        "GIT_COMMITTER_NAME" => committer.name, "GIT_COMMITTER_EMAIL" => committer.email
      }
      git("commit-tree", tree, *parents.flat_map { |parent_| ["-p", parent_] }, "-m", message, env: env).strip
    end

    def create_ref(ref, sha)
      git("update-ref", ref, sha, "")
    end

    def resolve(branch)
      out, status = capture("rev-parse", "--verify", "--quiet", "refs/heads/#{branch}^{commit}")
      out.strip if status.success?
    end

    def ls_tree(treeish)
      git("ls-tree", "-z", treeish).split("\0").map do |line_|
        meta, name = line_.split("\t", 2)
        Entry.new(*meta.split(" "), name)
      end
    end

    def cat_blob(sha)
      git("cat-file", "blob", sha)
    end

    def blob_at(commit, file_path)
      out, status = capture("rev-parse", "--verify", "--quiet", "#{commit}:#{file_path}")
      return unless status.success?

      sha = out.strip
      cat_blob(sha) if git("cat-file", "-t", sha).strip == "blob"
    end

    private

    def git(*args, stdin: nil, env: {}, git_dir: true)
      out, status, err = capture(*args, stdin: stdin, env: env, git_dir: git_dir)
      raise Error, "git #{args.first} failed: #{err.strip}" unless status.success?

      out
    end

    def capture(*args, stdin: nil, env: {}, git_dir: true)
      command = ["git", *(git_dir ? ["--git-dir=#{path}"] : []), *args]
      out, err, status = Open3.capture3(env, *command, stdin_data: stdin.to_s, binmode: true)
      [out, status, err]
    end
  end
end
