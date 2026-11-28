module Organisms
  module Sidebar
    class Component < ViewComponent::Base
      def initialize(workspace:, user:, expanded_slug: nil)
        @workspace = workspace
        @user = user
        @expanded_slug = expanded_slug
      end

      private
      attr_reader :workspace, :user, :expanded_slug

      def expanded?(repository)
        repository.slug == (request.path_parameters[:repository_slug] || expanded_slug)
      end

      def current_page_in?(path)
        current_page?(path)
      end
    end
  end
end
