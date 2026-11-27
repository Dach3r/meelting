module Molecules
  module Tray
    class Component < ViewComponent::Base
      def initialize(title:, count: nil, preview: nil, link: nil, list: false, **html_options)
        @title = title
        @count = count
        @preview = preview
        @link = link
        @list = list
        @html_options = html_options
      end

      private
      attr_reader :title, :count, :preview, :link, :list, :html_options

      def body_tag
        list ? :ul : :div
      end
    end
  end
end
