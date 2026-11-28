module Organisms
  module Sidebar
    class Component < ViewComponent::Base
      def initialize(workspace:, user:)
        @workspace = workspace
        @user = user
      end

      private
      attr_reader :workspace, :user

      def current_page_in?(path)
        current_page?(path)
      end
    end
  end
end
