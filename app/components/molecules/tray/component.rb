module Molecules
  module Tray
    class Component < ViewComponent::Base
      def initialize(title:, count: nil, **html_options)
        @title = title
        @count = count
        @html_options = html_options
      end

      private
      attr_reader :title, :count, :html_options
    end
  end
end
