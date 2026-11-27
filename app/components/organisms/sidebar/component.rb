module Organisms
  module Sidebar
    class Component < ViewComponent::Base
      def initialize(workspace:, expanded_slug: nil)
        @workspace = workspace
        @expanded_slug = expanded_slug
      end

      private
      attr_reader :workspace, :expanded_slug

      def expanded?(repository)
        repository.slug == (request.path_parameters[:repository_slug] || expanded_slug)
      end

      def current_page_in?(path)
        current_page?(path)
      end
    end
  end
end
