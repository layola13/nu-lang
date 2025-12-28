#include <cstdint>
#include <string>
#include <iostream>
#include <vector>
#include <memory>
#include <optional>

void test_enumerate() {
    for(i, record) in self.history.iter().enumerate() {
        std::cout<< "Index: "<< i<< ", Record: "<< record<< std::endl;
    }
}
