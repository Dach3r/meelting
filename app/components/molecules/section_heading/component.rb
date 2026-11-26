module Molecules
  module SectionHeading
    class Component < ViewComponent::Base
      def initialize(label:, title:, title_dim: nil, aside: nil)
        @label = label
        @title = title
        @title_dim = title_dim
        @aside = aside
      end

      private
      attr_reader :label, :title, :title_dim, :aside
    end
  end
end
