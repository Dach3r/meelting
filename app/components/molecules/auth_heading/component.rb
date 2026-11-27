module Molecules
  module AuthHeading
    class Component < ViewComponent::Base
      def initialize(title:, title_dim: nil, description: nil)
        @title = title
        @title_dim = title_dim
        @description = description
      end

      private
      attr_reader :title, :title_dim, :description
    end
  end
end
