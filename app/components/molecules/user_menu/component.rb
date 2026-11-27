module Molecules
  module UserMenu
    class Component < ViewComponent::Base
      def initialize(user:)
        @user = user
      end

      private
      attr_reader :user
    end
  end
end
