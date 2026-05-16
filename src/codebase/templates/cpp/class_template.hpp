// @LITE_DESC C++ header-only class template with RAII, move semantics, operators, and iterator support
// @LITE_SCENE library
// @LITE_TAGS cpp,header,class,template,hpp

#ifndef CLASS_TEMPLATE_HPP
#define CLASS_TEMPLATE_HPP

#include <iostream>
#include <memory>
#include <iterator>
#include <algorithm>
#include <stdexcept>
#include <initializer_list>
#include <type_traits>
#include <functional>

/* Custom exception class */
template<typename T>
class ContainerException : public std::runtime_error {
public:
    explicit ContainerException(const std::string& message)
        : std::runtime_error(message) {}
};

/* Main container class template */
template<typename T>
class Container {
private:
    /* Internal node structure for linked storage */
    struct Node {
        T data;
        std::unique_ptr<Node> next;

        template<typename U>
        explicit Node(U&& value, std::unique_ptr<Node> nxt = nullptr)
            : data(std::forward<U>(value)), next(std::move(nxt)) {}
    };

    std::unique_ptr<Node> head_;
    size_t size_;

    /* Helper function for deep copy */
    std::unique_ptr<Node> copy_nodes(const Node* other) const {
        if (!other) return nullptr;

        auto new_node = std::make_unique<Node>(other->data);
        Node* current = new_node.get();
        other = other->next.get();

        while (other) {
            current->next = std::make_unique<Node>(other->data);
            current = current->next.get();
            other = other->next.get();
        }

        return new_node;
    }

public:
    /* ========== Type Aliases ========== */
    using value_type = T;
    using reference = T&;
    using const_reference = const T&;
    using pointer = T*;
    using const_pointer = const T*;
    using size_type = size_t;

    /* ========== Constructors ========== */

    // Default constructor
    Container() noexcept : head_(nullptr), size_(0) {}

    // Copy constructor
    Container(const Container& other)
        : head_(copy_nodes(other.head_.get())), size_(other.size_) {}

    // Move constructor
    Container(Container&& other) noexcept
        : head_(std::move(other.head_)), size_(other.size_) {
        other.size_ = 0;
    }

    // Initializer list constructor
    Container(std::initializer_list<T> init) : head_(nullptr), size_(0) {
        for (const auto& item : init) {
            push_back(item);
        }
    }

    // Range constructor
    template<typename InputIt>
    Container(InputIt first, InputIt last) : head_(nullptr), size_(0) {
        for (auto it = first; it != last; ++it) {
            push_back(*it);
        }
    }

    /* ========== Destructor ========== */
    ~Container() = default;

    /* ========== Assignment Operators ========== */

    // Copy assignment with copy-and-swap idiom
    Container& operator=(Container other) noexcept {
        swap(other);
        return *this;
    }

    // Move assignment
    Container& operator=(Container&& other) noexcept {
        if (this != &other) {
            head_ = std::move(other.head_);
            size_ = other.size_;
            other.size_ = 0;
        }
        return *this;
    }

    // Initializer list assignment
    Container& operator=(std::initializer_list<T> init) {
        Container temp(init);
        swap(temp);
        return *this;
    }

    /* ========== Element Access ========== */

    // Front element access
    reference front() {
        if (!head_) {
            throw ContainerException<T>("Container is empty");
        }
        return head_->data;
    }

    const_reference front() const {
        if (!head_) {
            throw ContainerException<T>("Container is empty");
        }
        return head_->data;
    }

    // Back element access
    reference back() {
        if (!head_) {
            throw ContainerException<T>("Container is empty");
        }
        Node* current = head_.get();
        while (current->next) {
            current = current->next.get();
        }
        return current->data;
    }

    const_reference back() const {
        if (!head_) {
            throw ContainerException<T>("Container is empty");
        }
        const Node* current = head_.get();
        while (current->next) {
            current = current->next.get();
        }
        return current->data;
    }

    // Element access by index (checked)
    reference at(size_type index) {
        if (index >= size_) {
            throw ContainerException<T>("Index out of range");
        }

        Node* current = head_.get();
        for (size_type i = 0; i < index; ++i) {
            current = current->next.get();
        }
        return current->data;
    }

    const_reference at(size_type index) const {
        if (index >= size_) {
            throw ContainerException<T>("Index out of range");
        }

        const Node* current = head_.get();
        for (size_type i = 0; i < index; ++i) {
            current = current->next.get();
        }
        return current->data;
    }

    // Unchecked element access
    reference operator[](size_type index) {
        Node* current = head_.get();
        for (size_type i = 0; i < index; ++i) {
            current = current->next.get();
        }
        return current->data;
    }

    const_reference operator[](size_type index) const {
        const Node* current = head_.get();
        for (size_type i = 0; i < index; ++i) {
            current = current->next.get();
        }
        return current->data;
    }

    /* ========== Iterators ========== */

    // Forward iterator
    class iterator {
    private:
        Node* current_;

    public:
        using iterator_category = std::forward_iterator_tag;
        using value_type = T;
        using difference_type = std::ptrdiff_t;
        using pointer = T*;
        using reference = T&;

        iterator() : current_(nullptr) {}
        explicit iterator(Node* node) : current_(node) {}

        reference operator*() const { return current_->data; }
        pointer operator->() const { return &current_->data; }

        iterator& operator++() {
            current_ = current_->next.get();
            return *this;
        }

        iterator operator++(int) {
            iterator temp = *this;
            current_ = current_->next.get();
            return temp;
        }

        bool operator==(const iterator& other) const {
            return current_ == other.current_;
        }

        bool operator!=(const iterator& other) const {
            return current_ != other.current_;
        }

        friend class Container;
    };

    // Const iterator
    class const_iterator {
    private:
        const Node* current_;

    public:
        using iterator_category = std::forward_iterator_tag;
        using value_type = T;
        using difference_type = std::ptrdiff_t;
        using pointer = const T*;
        using reference = const T&;

        const_iterator() : current_(nullptr) {}
        explicit const_iterator(const Node* node) : current_(node) {}
        const_iterator(const iterator& other) : current_(other.current_) {}

        reference operator*() const { return current_->data; }
        pointer operator->() const { return &current_->data; }

        const_iterator& operator++() {
            current_ = current_->next.get();
            return *this;
        }

        const_iterator operator++(int) {
            const_iterator temp = *this;
            current_ = current_->next.get();
            return temp;
        }

        bool operator==(const const_iterator& other) const {
            return current_ == other.current_;
        }

        bool operator!=(const const_iterator& other) const {
            return current_ != other.current_;
        }

        friend class Container;
    };

    // Iterator methods
    iterator begin() { return iterator(head_.get()); }
    iterator end() { return iterator(nullptr); }
    const_iterator begin() const { return const_iterator(head_.get()); }
    const_iterator end() const { return const_iterator(nullptr); }
    const_iterator cbegin() const { return const_iterator(head_.get()); }
    const_iterator cend() const { return const_iterator(nullptr); }

    /* ========== Capacity ========== */

    bool empty() const noexcept { return size_ == 0; }
    size_type size() const noexcept { return size_; }

    /* ========== Modifiers ========== */

    // Clear all elements
    void clear() noexcept {
        head_.reset();
        size_ = 0;
    }

    // Push element to front (perfect forwarding)
    template<typename U>
    void push_front(U&& value) {
        auto new_node = std::make_unique<Node>(std::forward<U>(value), std::move(head_));
        head_ = std::move(new_node);
        ++size_;
    }

    // Push element to back (perfect forwarding)
    template<typename U>
    void push_back(U&& value) {
        auto new_node = std::make_unique<Node>(std::forward<U>(value));

        if (!head_) {
            head_ = std::move(new_node);
        } else {
            Node* current = head_.get();
            while (current->next) {
                current = current->next.get();
            }
            current->next = std::move(new_node);
        }
        ++size_;
    }

    // Pop front element
    void pop_front() {
        if (!head_) {
            throw ContainerException<T>("Cannot pop from empty container");
        }
        head_ = std::move(head_->next);
        --size_;
    }

    // Insert element at position
    iterator insert(const_iterator pos, const T& value) {
        if (pos == cbegin()) {
            push_front(value);
            return begin();
        }

        Node* current = head_.get();
        while (current->next.get() != pos.current_) {
            current = current->next.get();
        }

        auto new_node = std::make_unique<Node>(value, std::move(current->next));
        current->next = std::move(new_node);
        ++size_;

        return iterator(current->next.get());
    }

    // Erase element at position
    iterator erase(const_iterator pos) {
        if (pos == cbegin()) {
            pop_front();
            return begin();
        }

        Node* current = head_.get();
        while (current->next.get() != pos.current_) {
            current = current->next.get();
        }

        current->next = std::move(current->next->next);
        --size_;

        return iterator(current->next.get());
    }

    /* ========== Operations ========== */

    // Swap contents
    void swap(Container& other) noexcept {
        using std::swap;
        swap(head_, other.head_);
        swap(size_, other.size_);
    }

    // Find element
    iterator find(const T& value) {
        Node* current = head_.get();
        while (current) {
            if (current->data == value) {
                return iterator(current);
            }
            current = current->next.get();
        }
        return end();
    }

    const_iterator find(const T& value) const {
        const Node* current = head_.get();
        while (current) {
            if (current->data == value) {
                return const_iterator(current);
            }
            current = current->next.get();
        }
        return cend();
    }

    // Check if element exists
    bool contains(const T& value) const {
        return find(value) != cend();
    }

    /* ========== Comparison Operators ========== */

    bool operator==(const Container& other) const {
        if (size_ != other.size_) {
            return false;
        }

        const Node* current = head_.get();
        const Node* other_current = other.head_.get();

        while (current && other_current) {
            if (current->data != other_current->data) {
                return false;
            }
            current = current->next.get();
            other_current = other_current->next.get();
        }

        return true;
    }

    bool operator!=(const Container& other) const {
        return !(*this == other);
    }

    /* ========== Stream Operators ========== */

    friend std::ostream& operator<<(std::ostream& os, const Container& container) {
        os << "[";
        const Node* current = container.head_.get();
        while (current) {
            os << current->data;
            if (current->next) {
                os << ", ";
            }
            current = current->next.get();
        }
        os << "]";
        return os;
    }

    friend std::istream& operator>>(std::istream& is, Container& container) {
        container.clear();
        T value;
        while (is >> value) {
            container.push_back(std::move(value));
        }
        return is;
    }
};

/* Non-member swap function */
template<typename T>
void swap(Container<T>& lhs, Container<T>& rhs) noexcept {
    lhs.swap(rhs);
}

/* Non-member erase_if */
template<typename T, typename Pred>
typename Container<T>::size_type erase_if(Container<T>& container, Pred pred) {
    auto original_size = container.size();

    auto it = container.begin();
    while (it != container.end()) {
        if (pred(*it)) {
            it = container.erase(it);
        } else {
            ++it;
        }
    }

    return original_size - container.size();
}

#endif // CLASS_TEMPLATE_HPP
