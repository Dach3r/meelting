module Molecules
  module SidebarRepository
    class Component < ViewComponent::Base
      SECTIONS = %i[code issues pull_requests runs memory].freeze
      MIRROR_TONES = { ok: :ok, failing: :bad, none: :none }.freeze

      def initialize(repository:, expanded:)
        @repository = repository
        @expanded = expanded
      end

      private
      attr_reader :repository, :expanded

      def monogram
        repository.name[0].downcase
      end

      def panel_id
        "sidebar-repository-#{repository.slug}"
      end

      def current_section?(section)
        repository.real && current_page?(section_path(section))
      end

      def section_path(section)
        return user_repositories_path(repository.username) unless repository.real

        section == :code ? repository_path(repository.username, repository.slug) : repository_section_path(repository.username, repository.slug, section)
      end
    end
  end
end
