module Molecules
  module SidebarRepository
    class Component < ViewComponent::Base
      SECTIONS = %i[code issues pull_requests runs memory].freeze
      MIRROR_TONES = { ok: :ok, failing: :bad, none: :none }.freeze

      def initialize(repository:, expanded: false, compact: false)
        @repository = repository
        @expanded = expanded
        @compact = compact
      end

      private
      attr_reader :repository, :expanded, :compact

      def monogram
        repository.name[0].downcase
      end

      def panel_id
        "sidebar-repository-#{repository.slug}"
      end

      def current_repository?
        repository.real && request.path_parameters[:repository_slug] == repository.slug
      end

      def current_section?(section)
        repository.real && current_page?(section_path(section))
      end

      def section_path(section)
        return user_repositories_path(repository.username) unless repository.real

        section == :code ? repository_path(repository.username, repository.slug) : repository_section_path(repository.username, repository.slug, section)
      end

      def rail_classes
        class_names(
          "relative flex h-[50px] w-[68px] shrink-0 items-center justify-center rounded-xl",
          current_repository? ? "bg-forest-active" : "hover:bg-forest-active/50"
        )
      end
    end
  end
end
