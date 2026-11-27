module Organisms
  module Navbar
    class Component < ViewComponent::Base
      def initialize(authenticated:, user: nil, counts: nil)
        @authenticated = authenticated
        @user = user
        @counts = counts
      end

      private
      attr_reader :authenticated, :user, :counts
    end
  end
end
