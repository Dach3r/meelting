module Repositories
  # Detects the license from the best-named license file at the root of the default branch.
  # The repository content is the source of truth; the result is cached by commit SHA, so a
  # push that changes the file is picked up without a hook.
  class DetectLicense
    Result = Data.define(:key, :path) do
      def other?
        key == "other"
      end

      def spdx_id
        metadata.spdx_id unless other?
      end

      def name
        metadata.name
      end

      def description
        metadata.meta.description
      end

      def rules
        %i[permissions conditions limitations].index_with { |group_| metadata.rules.public_send(group_).map(&:tag) }
      end

      private
      def metadata
        Licensee::License.find(key)
      end
    end

    def self.call(repository)
      new(repository).call
    end

    def initialize(repository)
      @repository = repository
    end

    def call
      sha = repository.head_sha
      return unless sha

      attributes = Rails.cache.fetch(["repository_license", repository.id, sha]) { detect(sha) }
      Result.new(**attributes) if attributes
    end

    private
    attr_reader :repository

    def detect(sha)
      file = best_candidate(sha)
      return unless file

      content = repository.git.cat_blob(file.sha)
      { key: Licensee::ProjectFiles::LicenseFile.new(content, name: file.name).license.key, path: file.name }
    end

    def best_candidate(sha)
      scored = repository.git.ls_tree(sha).select { |entry_| entry_.type == "blob" }.filter_map do |entry_|
        score = Licensee::ProjectFiles::LicenseFile.name_score(entry_.name)
        [entry_, score] if score.positive?
      end
      scored.min_by { |entry_, score_| [-score_, entry_.name] }&.first
    end
  end
end
