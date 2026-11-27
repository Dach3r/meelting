module Molecules
  module AuthHeading
    class Component < ViewComponent::Base
      def initialize(title:, description: nil)
        @title = title
        @description = description
      end

      private
      attr_reader :title, :description
    end
  end
end
