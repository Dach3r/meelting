module Molecules
  module Panel
    class Component < ViewComponent::Base
      def initialize(title:, meta: nil)
        @title = title
        @meta = meta
      end

      private
      attr_reader :title, :meta
    end
  end
end
