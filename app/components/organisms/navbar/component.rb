module Organisms
  module Navbar
    class Component < ViewComponent::Base
      def initialize(authenticated:, user: nil)
        @authenticated = authenticated
        @user = user
      end

      private
      attr_reader :authenticated, :user
    end
  end
end
